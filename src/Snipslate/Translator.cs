using System.Net;
using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;

namespace Snipslate;

public enum Provider
{
    Anthropic,
    OpenAI,
    OpenRouter,
    Custom,
}

/// <summary>One translation request. Exactly one of <see cref="Png"/> and <see cref="Text"/> is set.</summary>
public sealed record TranslateJob(
    Provider Provider,
    string BaseUrl,
    string Model,
    string ApiKey,
    string TargetLanguage,
    byte[]? Png,
    string? Text);

/// <summary>A failure with a message that is safe and useful to show the user as-is.</summary>
public sealed class TranslateException(string message) : Exception(message);

/// <summary>
/// Calls the user's chosen AI provider. Anthropic uses the Messages API; OpenAI, OpenRouter and
/// Custom share the OpenAI chat-completions format, so they differ only by base URL.
/// </summary>
public static partial class Translator
{
    public const string AnthropicVersion = "2026-10-21";
    public const string FallbackBeta = "server-side-fallback-2026-07-01";
    public static readonly TimeSpan Timeout = TimeSpan.FromSeconds(60);

    private static readonly HttpClient Http = new() { Timeout = Timeout };

    public static string DisplayName(Provider provider) => provider switch
    {
        Provider.Anthropic => "Anthropic",
        Provider.OpenAI => "OpenAI",
        Provider.OpenRouter => "OpenRouter",
        _ => "Custom (OpenAI-compatible)",
    };

    public static string DefaultModel(Provider provider) => provider switch
    {
        Provider.Anthropic => "claude-opus-5-5",
        Provider.OpenAI => "gpt-5-mini",
        Provider.OpenRouter => "openai/gpt-5-mini",
        _ => "",
    };

    public static string DefaultBaseUrl(Provider provider) => provider switch
    {
        Provider.Anthropic => "https://api.anthropic.com/v1",
        Provider.OpenAI => "https://api.openai.com/v1",
        Provider.OpenRouter => "https://openrouter.ai/api/v1",
        _ => "http://localhost:11434/v1",
    };

    /// <summary>API keys only travel over HTTPS. Plain HTTP is allowed for a server on this machine.</summary>
    public static bool IsAllowedBaseUrl(string url) =>
        Uri.TryCreate(url, UriKind.Absolute, out var uri)
        && (uri.Scheme == Uri.UriSchemeHttps || (uri.Scheme == Uri.UriSchemeHttp && uri.IsLoopback));

    public static async Task<string> TranslateAsync(TranslateJob job, CancellationToken cancellation = default)
    {
        using var request = BuildRequest(job);
        try
        {
            using var response = await Http.SendAsync(request, cancellation);
            var body = await response.Content.ReadAsStringAsync(cancellation);
            return ParseResponse(job.Provider, response.StatusCode, body);
        }
        catch (TaskCanceledException) when (!cancellation.IsCancellationRequested)
        {
            throw new TranslateException($"The provider took longer than {Timeout.TotalSeconds:0} seconds to answer. Try again.");
        }
        catch (HttpRequestException ex)
        {
            throw new TranslateException(
                $"Couldn't reach {request.RequestUri?.Host}. Check your internet connection and the base URL.\n\n{ex.Message}");
        }
    }

    public static HttpRequestMessage BuildRequest(TranslateJob job)
    {
        var baseUrl = job.BaseUrl.TrimEnd('/');
        var request = job.Provider == Provider.Anthropic
            ? new HttpRequestMessage(HttpMethod.Post, baseUrl + "/messages")
            : new HttpRequestMessage(HttpMethod.Post, baseUrl + "/chat/completions");

        if (job.Provider == Provider.Anthropic)
        {
            request.Headers.Add("x-api-key", job.ApiKey);
            request.Headers.Add("anthropic-version", AnthropicVersion);
            if (SupportsFallback(job.Model))
            {
                request.Headers.Add("anthropic-beta", FallbackBeta);
            }
        }
        else if (job.ApiKey.Length > 0)
        {
            request.Headers.Authorization = new AuthenticationHeaderValue("Bearer", job.ApiKey);
        }

        request.Content = new StringContent(BuildBody(job).ToJsonString(), Encoding.UTF8, "application/json");
        return request;
    }

    public static JsonObject BuildBody(TranslateJob job)
    {
        var instructions = Instructions(job.TargetLanguage);
        var prompt = job.Png is not null
            ? $"Translate the text in this image into {job.TargetLanguage}."
            : $"Translate this text into {job.TargetLanguage}:\n\n{job.Text}";

        if (job.Provider == Provider.Anthropic)
        {
            var content = new JsonArray();
            if (job.Png is not null)
            {
                content.Add(new JsonObject
                {
                    ["type"] = "image",
                    ["source"] = new JsonObject
                    {
                        ["type"] = "base64",
                        ["media_type"] = "image/png",
                        ["data"] = Convert.ToBase64String(job.Png),
                    },
                });
            }

            content.Add(new JsonObject { ["type"] = "text", ["text"] = prompt });
            var body = new JsonObject
            {
                ["model"] = job.Model,
                ["max_tokens"] = 8192,
                ["system"] = instructions,
                ["messages"] = new JsonArray(new JsonObject { ["role"] = "user", ["content"] = content }),
            };

            // Translation is a simple task: low effort keeps the popup fast. Older models reject the field.
            if (IsClaude5(job.Model))
            {
                body["output_config"] = new JsonObject { ["effort"] = "low" };
            }

            if (SupportsFallback(job.Model))
            {
                body["fallbacks"] = "default";
            }

            return body;
        }

        JsonNode userContent = job.Png is null
            ? JsonValue.Create(prompt)!
            : new JsonArray(
                new JsonObject { ["type"] = "text", ["text"] = prompt },
                new JsonObject
                {
                    ["type"] = "image_url",
                    ["image_url"] = new JsonObject { ["url"] = "data:image/png;base64," + Convert.ToBase64String(job.Png) },
                });

        return new JsonObject
        {
            ["model"] = job.Model,
            ["messages"] = new JsonArray(
                new JsonObject { ["role"] = "system", ["content"] = instructions },
                new JsonObject { ["role"] = "user", ["content"] = userContent }),
        };
    }

    public static string ParseResponse(Provider provider, HttpStatusCode status, string body)
    {
        bool ok = (int)status is >= 200 and < 300;
        JsonElement root = default;
        try
        {
            using var document = JsonDocument.Parse(body);
            root = document.RootElement.Clone();
        }
        catch (JsonException)
        {
            if (ok)
            {
                throw new TranslateException("The provider sent a reply Snipslate couldn't read.");
            }
        }

        if (!ok)
        {
            throw new TranslateException(DescribeError(status, ErrorMessage(root)));
        }

        if (root.ValueKind != JsonValueKind.Object)
        {
            throw new TranslateException("The provider sent a reply Snipslate couldn't read.");
        }

        // OpenRouter can report an upstream failure inside an HTTP 200.
        if (ErrorMessage(root) is { } error)
        {
            throw new TranslateException($"The provider returned an error.\n\nProvider said: {error}");
        }

        return provider == Provider.Anthropic ? ReadAnthropic(root) : ReadOpenAI(root);
    }

    public static string DescribeError(HttpStatusCode status, string? detail)
    {
        var summary = (int)status switch
        {
            401 or 403 => "The API key was rejected. Check it in Settings.",
            404 => "Not found. Check the model name and base URL in Settings.",
            429 => "Rate limited or out of credit. Wait a moment, or check your account balance.",
            >= 500 => "The provider had a server error. Try again in a moment.",
            _ => $"The provider refused the request (HTTP {(int)status}).",
        };
        return string.IsNullOrWhiteSpace(detail) ? summary : $"{summary}\n\nProvider said: {detail}";
    }

    internal static string Instructions(string targetLanguage) =>
        $"You translate text captured from a screenshot. Reply with only the translation into {targetLanguage}: "
        + "no notes, no quotes, no preamble. Keep the original line breaks. "
        + $"If the text is already in {targetLanguage}, reply with it unchanged. "
        + "If there is no readable text, reply with exactly: (no text found)";

    internal static bool IsClaude5(string model) => Claude5().IsMatch(model);

    internal static bool SupportsFallback(string model) =>
        model is "claude-opus-5-5" or "claude-opus-5" or "claude-fable-5-1" or "claude-sonnet-5-5";

    private static string ReadAnthropic(JsonElement root)
    {
        if (root.TryGetProperty("stop_reason", out var stop) && stop.ValueKind == JsonValueKind.String
            && stop.GetString() == "refusal")
        {
            throw new TranslateException("The model declined to translate this snip.");
        }

        var text = new StringBuilder();
        if (root.TryGetProperty("content", out var content) && content.ValueKind == JsonValueKind.Array)
        {
            foreach (var block in content.EnumerateArray())
            {
                if (block.ValueKind == JsonValueKind.Object
                    && block.TryGetProperty("type", out var type) && type.ValueKind == JsonValueKind.String
                    && type.GetString() == "text"
                    && block.TryGetProperty("text", out var part) && part.ValueKind == JsonValueKind.String)
                {
                    text.Append(part.GetString());
                }
            }
        }

        return Finish(text.ToString());
    }

    private static string ReadOpenAI(JsonElement root)
    {
        if (root.TryGetProperty("choices", out var choices) && choices.ValueKind == JsonValueKind.Array
            && choices.GetArrayLength() > 0 && choices[0].ValueKind == JsonValueKind.Object
            && choices[0].TryGetProperty("message", out var message) && message.ValueKind == JsonValueKind.Object)
        {
            if (message.TryGetProperty("content", out var content) && content.ValueKind == JsonValueKind.String)
            {
                return Finish(content.GetString() ?? "");
            }

            if (message.TryGetProperty("refusal", out var refusal) && refusal.ValueKind == JsonValueKind.String)
            {
                throw new TranslateException($"The model declined to translate this snip.\n\nIt said: {refusal.GetString()}");
            }
        }

        throw new TranslateException("The provider's reply had no translation in it.");
    }

    private static string? ErrorMessage(JsonElement root)
    {
        if (root.ValueKind != JsonValueKind.Object || !root.TryGetProperty("error", out var error))
        {
            return null;
        }

        return error.ValueKind switch
        {
            JsonValueKind.String => error.GetString(),
            JsonValueKind.Object when error.TryGetProperty("message", out var m) && m.ValueKind == JsonValueKind.String
                => m.GetString(),
            _ => null,
        };
    }

    private static string Finish(string text)
    {
        text = text.Trim();
        return text.Length > 0 ? text : throw new TranslateException("The model returned an empty reply.");
    }

    [GeneratedRegex("^claude-(opus|sonnet|haiku|fable|mythos)-5(-|$)")]
    private static partial Regex Claude5();
}

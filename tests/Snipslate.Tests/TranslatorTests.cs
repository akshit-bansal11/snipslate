using System.Net;
using System.Text.Json;

namespace Snipslate.Tests;

public class TranslatorTests
{
    private static readonly byte[] Png = [1, 2, 3];

    private static TranslateJob Job(Provider provider, string? model = null, byte[]? png = null, string? text = null, string key = "secret") =>
        new(provider, Translator.DefaultBaseUrl(provider), model ?? Translator.DefaultModel(provider), key, "English", png, text);

    [Fact]
    public void AnthropicRequestSendsTheImageWithVersionAndFallback()
    {
        var job = Job(Provider.Anthropic, png: Png);
        using var request = Translator.BuildRequest(job);

        Assert.Equal("https://api.anthropic.com/v1/messages", request.RequestUri?.ToString());
        Assert.Equal("secret", request.Headers.GetValues("x-api-key").Single());
        Assert.Equal(Translator.AnthropicVersion, request.Headers.GetValues("anthropic-version").Single());
        Assert.Equal(Translator.FallbackBeta, request.Headers.GetValues("anthropic-beta").Single());
        Assert.Null(request.Headers.Authorization);

        var body = Translator.BuildBody(job);
        Assert.Equal("claude-opus-5-5", (string?)body["model"]);
        Assert.Equal("default", (string?)body["fallbacks"]);
        Assert.Equal("low", (string?)body["output_config"]?["effort"]);
        Assert.Null(body["temperature"]);
        var content = body["messages"]![0]!["content"]!.AsArray();
        Assert.Equal("image", (string?)content[0]!["type"]);
        Assert.Equal("image/png", (string?)content[0]!["source"]!["media_type"]);
        Assert.Equal("AQID", (string?)content[0]!["source"]!["data"]);
        Assert.Equal("text", (string?)content[1]!["type"]);
    }

    [Fact]
    public void OlderClaudeModelsGetNoEffortOrFallback()
    {
        var job = Job(Provider.Anthropic, model: "claude-haiku-4-5", png: Png);
        using var request = Translator.BuildRequest(job);
        Assert.False(request.Headers.Contains("anthropic-beta"));

        var body = Translator.BuildBody(job);
        Assert.Null(body["output_config"]);
        Assert.Null(body["fallbacks"]);
    }

    [Fact]
    public void HaikuFiveGetsEffortButNoFallback()
    {
        var body = Translator.BuildBody(Job(Provider.Anthropic, model: "claude-haiku-5-5", png: Png));
        Assert.Equal("low", (string?)body["output_config"]?["effort"]);
        Assert.Null(body["fallbacks"]);
    }

    [Theory]
    [InlineData(Provider.OpenAI, "https://api.openai.com/v1/chat/completions")]
    [InlineData(Provider.OpenRouter, "https://openrouter.ai/api/v1/chat/completions")]
    public void OpenAiCompatibleRequestSendsADataUrl(Provider provider, string url)
    {
        var job = Job(provider, png: Png);
        using var request = Translator.BuildRequest(job);
        Assert.Equal(url, request.RequestUri?.ToString());
        Assert.Equal("Bearer", request.Headers.Authorization?.Scheme);
        Assert.Equal("secret", request.Headers.Authorization?.Parameter);

        var messages = Translator.BuildBody(job)["messages"]!.AsArray();
        Assert.Equal("system", (string?)messages[0]!["role"]);
        var content = messages[1]!["content"]!.AsArray();
        Assert.Equal("image_url", (string?)content[1]!["type"]);
        Assert.Equal("data:image/png;base64,AQID", (string?)content[1]!["image_url"]!["url"]);
    }

    [Fact]
    public void TextOnlySendsPlainStringContent()
    {
        var body = Translator.BuildBody(Job(Provider.OpenAI, text: "Hola"));
        var content = body["messages"]![1]!["content"]!;
        Assert.Equal(JsonValueKind.String, content.GetValueKind());
        Assert.Contains("Hola", (string?)content);
    }

    [Fact]
    public void CustomWithoutKeySendsNoAuthorization()
    {
        var job = Job(Provider.Custom, model: "llama3.2-vision", png: Png, key: "") with { BaseUrl = "http://localhost:11434/v1/" };
        using var request = Translator.BuildRequest(job);
        Assert.Equal("http://localhost:11434/v1/chat/completions", request.RequestUri?.ToString());
        Assert.Null(request.Headers.Authorization);
    }

    [Theory]
    [InlineData("https://example.com/v1", true)]
    [InlineData("http://localhost:1234/v1", true)]
    [InlineData("http://127.0.0.1:8080", true)]
    [InlineData("http://example.com/v1", false)]
    [InlineData("ftp://example.com", false)]
    [InlineData("not a url", false)]
    [InlineData("", false)]
    public void BaseUrlMustBeHttpsOrLocal(string url, bool allowed) =>
        Assert.Equal(allowed, Translator.IsAllowedBaseUrl(url));

    [Fact]
    public async Task RefusesToSendAKeyOverPlainHttpToAnotherHost()
    {
        var job = Job(Provider.Custom, model: "m", text: "Hola") with { BaseUrl = "http://example.com/v1" };
        var ex = await Assert.ThrowsAsync<TranslateException>(() =>
            Translator.TranslateAsync(job, TestContext.Current.CancellationToken));
        Assert.Contains("https://", ex.Message);
    }

    [Fact]
    public void JobToStringHidesTheKey() =>
        Assert.DoesNotContain("secret", Job(Provider.OpenAI, text: "x").ToString());

    [Fact]
    public void ReadsAnthropicTextBlocks()
    {
        const string body = """
            {"stop_reason":"end_turn","content":[{"type":"thinking","thinking":""},{"type":"text","text":" Hello "},{"type":"text","text":"world"}]}
            """;
        Assert.Equal("Hello world", Translator.ParseResponse(Provider.Anthropic, HttpStatusCode.OK, body));
    }

    [Fact]
    public void AnthropicRefusalIsAnError()
    {
        const string body = """{"stop_reason":"refusal","content":[]}""";
        var ex = Assert.Throws<TranslateException>(() => Translator.ParseResponse(Provider.Anthropic, HttpStatusCode.OK, body));
        Assert.Contains("declined", ex.Message);
    }

    [Fact]
    public void ReadsOpenAiMessageContent()
    {
        const string body = """{"choices":[{"message":{"role":"assistant","content":"Hello world\n"}}]}""";
        Assert.Equal("Hello world", Translator.ParseResponse(Provider.OpenAI, HttpStatusCode.OK, body));
    }

    [Fact]
    public void ErrorInsideHttp200IsReported()
    {
        const string body = """{"error":{"message":"Provider returned error","code":502}}""";
        var ex = Assert.Throws<TranslateException>(() => Translator.ParseResponse(Provider.OpenRouter, HttpStatusCode.OK, body));
        Assert.Contains("Provider returned error", ex.Message);
    }

    [Fact]
    public void RejectedKeyNamesTheKey()
    {
        const string body = """{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}""";
        var ex = Assert.Throws<TranslateException>(() => Translator.ParseResponse(Provider.Anthropic, HttpStatusCode.Unauthorized, body));
        Assert.Contains("API key", ex.Message);
        Assert.Contains("invalid x-api-key", ex.Message);
    }

    [Fact]
    public void NonJsonServerErrorStillExplainsItself()
    {
        var ex = Assert.Throws<TranslateException>(() =>
            Translator.ParseResponse(Provider.OpenAI, HttpStatusCode.BadGateway, "<html>bad gateway</html>"));
        Assert.Contains("server error", ex.Message);
    }

    [Fact]
    public void EmptyReplyIsAnError() =>
        Assert.Throws<TranslateException>(() =>
            Translator.ParseResponse(Provider.OpenAI, HttpStatusCode.OK, """{"choices":[{"message":{"content":"  "}}]}"""));
}

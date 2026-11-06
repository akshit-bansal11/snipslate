using System.Text.Json;
using System.Text.Json.Serialization;
using Windows.Security.Credentials;

namespace Snipslate;

/// <summary>Everything except API keys. Saved as JSON in %APPDATA%\Snipslate.</summary>
public sealed class AppSettings
{
    public Provider Provider { get; set; } = Provider.Anthropic;

    public string Model { get; set; } = Translator.DefaultModel(Provider.Anthropic);

    public string CustomBaseUrl { get; set; } = Translator.DefaultBaseUrl(Provider.Custom);

    public string TargetLanguage { get; set; } = "English";

    public bool TextOnly { get; set; }

    public string TranslateHotkey { get; set; } = "Ctrl+Alt+T";

    public string CopyHotkey { get; set; } = "Ctrl+Alt+C";

    public string ResolveBaseUrl() =>
        Provider == Provider.Custom ? CustomBaseUrl : Translator.DefaultBaseUrl(Provider);
}

internal static class SettingsStore
{
    private static readonly string Folder =
        Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "Snipslate");

    private static readonly string FilePath = Path.Combine(Folder, "settings.json");

    private static readonly JsonSerializerOptions Options = new()
    {
        WriteIndented = true,
        Converters = { new JsonStringEnumConverter() },
    };

    /// <summary>Loads settings; a missing or unreadable file gives the defaults (it holds no keys, so nothing is lost).</summary>
    public static AppSettings Load()
    {
        try
        {
            return File.Exists(FilePath)
                ? JsonSerializer.Deserialize<AppSettings>(File.ReadAllText(FilePath), Options) ?? new AppSettings()
                : new AppSettings();
        }
        catch (Exception ex) when (ex is IOException or JsonException or UnauthorizedAccessException)
        {
            return new AppSettings();
        }
    }

    public static void Save(AppSettings settings)
    {
        Directory.CreateDirectory(Folder);
        var temp = FilePath + ".tmp";
        File.WriteAllText(temp, JsonSerializer.Serialize(settings, Options));
        File.Move(temp, FilePath, overwrite: true);
    }
}

/// <summary>API keys live in Windows Credential Manager, one per provider, never in the settings file.</summary>
internal static class KeyStore
{
    private const string Resource = "Snipslate";

    public static string? Get(Provider provider)
    {
        try
        {
            var credential = new PasswordVault().Retrieve(Resource, provider.ToString());
            credential.RetrievePassword();
            return string.IsNullOrEmpty(credential.Password) ? null : credential.Password;
        }
        catch (Exception)
        {
            // Retrieve throws when no key is stored for this provider.
            return null;
        }
    }

    /// <summary>Add replaces an existing key for the same provider, so a failed save never loses the old one.</summary>
    public static void Set(Provider provider, string key) =>
        new PasswordVault().Add(new PasswordCredential(Resource, provider.ToString(), key));

    public static void Remove(Provider provider)
    {
        try
        {
            var vault = new PasswordVault();
            vault.Remove(vault.Retrieve(Resource, provider.ToString()));
        }
        catch (Exception)
        {
            // Nothing stored: nothing to remove.
        }
    }
}

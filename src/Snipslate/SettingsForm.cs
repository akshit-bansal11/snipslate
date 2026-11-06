using System.Windows.Forms.Automation;

namespace Snipslate;

internal sealed class SettingsForm : Form
{
    private static readonly string[] Languages =
    [
        "English", "Spanish", "French", "German", "Italian", "Portuguese", "Dutch", "Russian", "Ukrainian",
        "Polish", "Turkish", "Arabic", "Hindi", "Bengali", "Urdu", "Chinese (Simplified)", "Chinese (Traditional)",
        "Japanese", "Korean", "Vietnamese", "Thai", "Indonesian",
    ];

    private readonly AppSettings settings;
    private readonly ComboBox provider = new() { DropDownStyle = ComboBoxStyle.DropDownList, Dock = DockStyle.Fill };
    private readonly TextBox baseUrl = new() { Dock = DockStyle.Fill };
    private readonly TextBox model = new() { Dock = DockStyle.Fill };
    private readonly TextBox apiKey = new() { Dock = DockStyle.Fill, UseSystemPasswordChar = true };
    private readonly Label keyState = new() { AutoSize = true, MaximumSize = new Size(320, 0) };
    private readonly Button test = new() { Text = "&Test", AutoSize = true };
    private readonly LinkLabel forget = new() { Text = "Forget saved key", AutoSize = true, Margin = new Padding(6, 8, 0, 0) };
    private readonly ComboBox language = new() { Dock = DockStyle.Fill };
    private readonly CheckBox textOnly = new() { Text = "Send text only (for models that can't read images)", AutoSize = true };
    private readonly TextBox translateHotkey = new() { Dock = DockStyle.Fill };
    private readonly TextBox copyHotkey = new() { Dock = DockStyle.Fill };
    private readonly Label message = new() { AutoSize = true, ForeColor = Color.Firebrick, MaximumSize = new Size(420, 0) };
    private Provider shown;
    private string customUrl;

    public SettingsForm(AppSettings settings)
    {
        this.settings = settings;
        customUrl = settings.CustomBaseUrl;

        Text = "Snipslate settings";
        FormBorderStyle = FormBorderStyle.FixedDialog;
        MaximizeBox = false;
        MinimizeBox = false;
        StartPosition = FormStartPosition.CenterScreen;
        AutoScaleMode = AutoScaleMode.Dpi;
        AutoScaleDimensions = new SizeF(96F, 96F);
        Font = SystemFonts.MessageBoxFont ?? Font;
        AutoSize = true;
        AutoSizeMode = AutoSizeMode.GrowAndShrink;

        var grid = new TableLayoutPanel { ColumnCount = 2, AutoSize = true, Padding = new Padding(12) };
        grid.ColumnStyles.Add(new ColumnStyle(SizeType.AutoSize));
        grid.ColumnStyles.Add(new ColumnStyle(SizeType.Absolute, 330F));

        void Row(string label, Control control)
        {
            grid.Controls.Add(new Label { Text = label, AutoSize = true, Anchor = AnchorStyles.Left, Margin = new Padding(0, 6, 12, 6) });
            grid.Controls.Add(control);
            control.AccessibleName = label.Replace("&", "");
        }

        foreach (var p in Enum.GetValues<Provider>())
        {
            provider.Items.Add(Translator.DisplayName(p));
        }

        language.Items.AddRange(Languages);
        apiKey.PlaceholderText = "Paste a key to save or replace it";

        var keyActions = new FlowLayoutPanel { AutoSize = true, Dock = DockStyle.Fill, Margin = Padding.Empty };
        keyActions.Controls.AddRange([test, forget]);
        var buttons = new FlowLayoutPanel { AutoSize = true, FlowDirection = FlowDirection.RightToLeft, Dock = DockStyle.Fill };
        var save = new Button { Text = "&Save", AutoSize = true };
        var cancel = new Button { Text = "Cancel", AutoSize = true, DialogResult = DialogResult.Cancel };
        buttons.Controls.AddRange([cancel, save]);

        Row("&Provider", provider);
        Row("&Base URL", baseUrl);
        Row("&Model", model);
        Row("API &key", apiKey);
        Row("", keyActions);
        Row("", keyState);
        Row("Translate &to", language);
        Row("", textOnly);
        Row("Translate &hotkey", translateHotkey);
        Row("Copy-text h&otkey", copyHotkey);
        Row("", message);
        Row("", buttons);
        Controls.Add(grid);

        AcceptButton = save;
        CancelButton = cancel;

        shown = settings.Provider;
        provider.SelectedIndex = (int)settings.Provider;
        model.Text = settings.Model;
        language.Text = settings.TargetLanguage;
        textOnly.Checked = settings.TextOnly;
        translateHotkey.Text = settings.TranslateHotkey;
        copyHotkey.Text = settings.CopyHotkey;
        ShowProvider(settings.Provider);

        provider.SelectedIndexChanged += (_, _) => SwitchProvider((Provider)provider.SelectedIndex);
        forget.LinkClicked += (_, _) =>
        {
            KeyStore.Remove(Selected);
            ShowProvider(Selected);
        };
        test.Click += async (_, _) => await TestAsync();
        save.Click += (_, _) => Save();
    }

    private Provider Selected => (Provider)provider.SelectedIndex;

    private void SwitchProvider(Provider next)
    {
        if (shown == Provider.Custom)
        {
            customUrl = baseUrl.Text.Trim();
        }

        if (model.Text.Trim().Length == 0 || model.Text.Trim() == Translator.DefaultModel(shown))
        {
            model.Text = Translator.DefaultModel(next);
        }

        apiKey.Text = "";
        shown = next;
        ShowProvider(next);
    }

    private void ShowProvider(Provider p)
    {
        baseUrl.ReadOnly = p != Provider.Custom;
        baseUrl.Text = p == Provider.Custom ? customUrl : Translator.DefaultBaseUrl(p);
        bool saved = KeyStore.Get(p) is not null;
        forget.Visible = saved;
        keyState.ForeColor = SystemColors.ControlText;
        keyState.Text = saved
            ? $"A key for {Translator.DisplayName(p)} is saved. Leave the box empty to keep it."
            : p == Provider.Custom
                ? "A key is optional for a local server such as Ollama or LM Studio."
                : $"No key saved for {Translator.DisplayName(p)} yet.";
    }

    private async Task TestAsync()
    {
        if (FindProblem(out _, out _) is { } problem)
        {
            Fail(problem);
            return;
        }

        var key = apiKey.Text.Trim().Length > 0 ? apiKey.Text.Trim() : KeyStore.Get(Selected) ?? "";
        if (key.Length == 0 && Selected != Provider.Custom)
        {
            Fail("Paste an API key first.");
            return;
        }

        test.Enabled = false;
        message.Text = "";
        keyState.ForeColor = SystemColors.ControlText;
        keyState.Text = "Testing…";
        try
        {
            var job = new TranslateJob(Selected, baseUrl.Text.Trim(), model.Text.Trim(), key, language.Text.Trim(), null, "Bonjour tout le monde");
            var reply = await Translator.TranslateAsync(job);
            keyState.Text = $"It works. The model replied: {reply}";
        }
        catch (TranslateException ex)
        {
            keyState.Text = "";
            Fail(ex.Message);
        }
        finally
        {
            if (!IsDisposed)
            {
                test.Enabled = true;
            }
        }
    }

    private void Save()
    {
        if (FindProblem(out var translateKey, out var copyKey) is { } problem)
        {
            Fail(problem);
            return;
        }

        try
        {
            if (apiKey.Text.Trim().Length > 0)
            {
                KeyStore.Set(Selected, apiKey.Text.Trim());
            }
        }
        catch (Exception ex)
        {
            Fail($"Couldn't save the key to Windows Credential Manager: {ex.Message}");
            return;
        }

        settings.Provider = Selected;
        settings.Model = model.Text.Trim();
        if (Selected == Provider.Custom)
        {
            settings.CustomBaseUrl = baseUrl.Text.Trim();
        }

        settings.TargetLanguage = language.Text.Trim();
        settings.TextOnly = textOnly.Checked;
        settings.TranslateHotkey = translateKey.ToString();
        settings.CopyHotkey = copyKey.ToString();

        try
        {
            SettingsStore.Save(settings);
        }
        catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
        {
            Fail($"Couldn't save settings: {ex.Message}");
            return;
        }

        DialogResult = DialogResult.OK;
    }

    /// <summary>Returns a message describing the first invalid field, or null when everything is valid.</summary>
    private string? FindProblem(out Hotkey translateKey, out Hotkey copyKey)
    {
        copyKey = default;
        if (!Hotkey.TryParse(translateHotkey.Text, out translateKey))
        {
            translateHotkey.Focus();
            return "The translate hotkey isn't valid. Use a modifier and a key, like Ctrl+Alt+T.";
        }

        if (!Hotkey.TryParse(copyHotkey.Text, out copyKey))
        {
            copyHotkey.Focus();
            return "The copy-text hotkey isn't valid. Use a modifier and a key, like Ctrl+Alt+C.";
        }

        if (translateKey == copyKey)
        {
            copyHotkey.Focus();
            return "The two hotkeys must be different.";
        }

        if (Selected == Provider.Custom && !Translator.IsAllowedBaseUrl(baseUrl.Text.Trim()))
        {
            baseUrl.Focus();
            return "The base URL must start with https:// (http:// is allowed only for localhost).";
        }

        if (model.Text.Trim().Length == 0)
        {
            model.Focus();
            return "Enter a model name.";
        }

        if (language.Text.Trim().Length == 0)
        {
            language.Focus();
            return "Choose a language to translate into.";
        }

        return null;
    }

    private void Fail(string text)
    {
        message.Text = text;

        // Screen readers don't watch label text, so announce the problem explicitly.
        message.AccessibilityObject.RaiseAutomationNotification(
            AutomationNotificationKind.Other, AutomationNotificationProcessing.ImportantMostRecent, text);
    }
}

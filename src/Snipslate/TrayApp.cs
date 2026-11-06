namespace Snipslate;

/// <summary>The tray icon, its menu and hotkeys, and the snip → copy text / translate flow.</summary>
internal sealed class TrayApp : ApplicationContext
{
    private const int TranslateId = 1;
    private const int CopyId = 2;

    private readonly NotifyIcon tray;
    private readonly HotkeyWindow hotkeys = new();
    private readonly ToolStripMenuItem translateItem;
    private readonly ToolStripMenuItem copyItem;
    private readonly AppSettings settings = SettingsStore.Load();
    private bool busy;
    private bool settingsOpen;

    public TrayApp()
    {
        translateItem = new ToolStripMenuItem("&Translate an area", null, (_, _) => RunSnip(translate: true));
        copyItem = new ToolStripMenuItem("&Copy text from an area", null, (_, _) => RunSnip(translate: false));
        var menu = new ContextMenuStrip();
        menu.Items.AddRange(new ToolStripItem[]
        {
            translateItem,
            copyItem,
            new ToolStripSeparator(),
            new ToolStripMenuItem("&Settings…", null, (_, _) => OpenSettings()),
            new ToolStripMenuItem("&Quit", null, (_, _) => ExitThread()),
        });

        tray = new NotifyIcon
        {
            Icon = SystemIcons.Application,
            Text = "Snipslate",
            ContextMenuStrip = menu,
            Visible = true,
        };
        tray.DoubleClick += (_, _) => RunSnip(translate: true);
        hotkeys.Pressed += id => RunSnip(translate: id == TranslateId);

        var failed = ApplyHotkeys();
        if (failed.Count > 0)
        {
            WarnHotkeys(failed);
        }
        else if (KeyStore.Get(settings.Provider) is null && settings.Provider != Provider.Custom)
        {
            Notify("Snipslate is running",
                $"Add an API key in Settings, then press {settings.TranslateHotkey} to translate an area. {settings.CopyHotkey} copies text.");
        }
        else
        {
            Notify("Snipslate is running", $"{settings.TranslateHotkey} translates an area. {settings.CopyHotkey} copies its text.");
        }
    }

    protected override void ExitThreadCore()
    {
        tray.Visible = false;
        tray.Dispose();
        hotkeys.Dispose();
        base.ExitThreadCore();
    }

    private List<string> ApplyHotkeys()
    {
        hotkeys.UnregisterAll();
        var failed = new List<string>();
        Register(TranslateId, settings.TranslateHotkey, translateItem, failed);
        Register(CopyId, settings.CopyHotkey, copyItem, failed);
        return failed;
    }

    private void Register(int id, string text, ToolStripMenuItem item, List<string> failed)
    {
        item.ShortcutKeyDisplayString = text;
        if (!Hotkey.TryParse(text, out var hotkey) || !hotkeys.Register(id, hotkey))
        {
            failed.Add(text);
        }
    }

    private void WarnHotkeys(List<string> failed) =>
        Notify("Hotkey unavailable",
            $"{string.Join(" and ", failed)} is already used by another app. Pick another in Settings; the tray menu still works.",
            ToolTipIcon.Warning);

    private async void RunSnip(bool translate)
    {
        if (busy)
        {
            return;
        }

        busy = true;
        try
        {
            // Let the tray menu close and the hotkey's keys lift before the screen is frozen.
            await Task.Delay(150);
            if (SnipOverlay.Pick() is not { } snip)
            {
                return;
            }

            using var image = snip.Image;
            if (translate)
            {
                await TranslateAsync(image, snip.ScreenRect);
            }
            else
            {
                await CopyTextAsync(image);
            }
        }
        catch (Exception ex)
        {
            MessageBox.Show(ex.Message, "Snipslate", MessageBoxButtons.OK, MessageBoxIcon.Error);
        }
        finally
        {
            busy = false;
        }
    }

    private async Task CopyTextAsync(Bitmap image)
    {
        var text = await Ocr.ReadTextAsync(image);
        if (text.Length == 0)
        {
            if (Imaging.IsMostlyBlack(image))
            {
                Notify("Nothing to read", "That area is black. Protected video (Netflix and other DRM players) can't be captured.", ToolTipIcon.Warning);
            }
            else
            {
                Notify("No text found", "Windows couldn't find any text in that area.");
            }

            return;
        }

        if (Clip.TrySet(text))
        {
            Notify("Text copied", $"{text.Length} characters copied to the clipboard.");
        }
        else
        {
            Notify("Clipboard busy", "Another app is holding the clipboard. Try again.", ToolTipIcon.Warning);
        }
    }

    private async Task TranslateAsync(Bitmap image, Rectangle where)
    {
        if (Imaging.IsMostlyBlack(image) && !ConfirmBlankSnip())
        {
            return;
        }

        var key = KeyStore.Get(settings.Provider);
        if (key is null && settings.Provider != Provider.Custom)
        {
            Notify("Add an API key", $"Snipslate needs a {Translator.DisplayName(settings.Provider)} API key to translate.");
            OpenSettings();
            return;
        }

        string? text = null;
        byte[]? png = null;
        if (settings.TextOnly)
        {
            text = await Ocr.ReadTextAsync(image);
            if (text.Length == 0)
            {
                Notify("No text found", "Windows couldn't find any text in that area.");
                return;
            }
        }
        else
        {
            png = Imaging.ToPng(image);
        }

        var job = new TranslateJob(settings.Provider, settings.ResolveBaseUrl(), settings.Model, key ?? "",
            settings.TargetLanguage, png, text);
        var popup = new ResultForm(where, $"{Translator.DisplayName(settings.Provider)} · {settings.Model}");
        popup.Show();

        // Not awaited: the next snip can start while this one is still being translated.
        _ = FillAsync(popup, job);
    }

    private static async Task FillAsync(ResultForm popup, TranslateJob job)
    {
        try
        {
            popup.ShowResult(await Translator.TranslateAsync(job, popup.Cancellation));
        }
        catch (OperationCanceledException)
        {
            // The popup was closed before the reply arrived.
        }
        catch (Exception ex)
        {
            popup.ShowError(ex.Message);
        }
    }

    private static bool ConfirmBlankSnip()
    {
        var send = new TaskDialogButton("Send anyway");
        var cancel = TaskDialogButton.Cancel;
        var page = new TaskDialogPage
        {
            Caption = "Snipslate",
            Heading = "This area looks blank",
            Text = "It's almost entirely black. Protected video (Netflix and other DRM players) always captures as black, "
                + "so there is nothing to translate. Tip: use the player's own subtitle settings.\n\n"
                + "If the area really has text on a black background, you can send it anyway.",
            Icon = TaskDialogIcon.Warning,
            Buttons = { send, cancel },
            DefaultButton = cancel,
        };
        return TaskDialog.ShowDialog(page) == send;
    }

    private void OpenSettings()
    {
        if (settingsOpen)
        {
            return;
        }

        settingsOpen = true;
        try
        {
            using var form = new SettingsForm(settings);
            if (form.ShowDialog() == DialogResult.OK)
            {
                var failed = ApplyHotkeys();
                if (failed.Count > 0)
                {
                    WarnHotkeys(failed);
                }
            }
        }
        finally
        {
            settingsOpen = false;
        }
    }

    private void Notify(string title, string text, ToolTipIcon icon = ToolTipIcon.Info) =>
        tray.ShowBalloonTip(5000, title, text, icon);
}

internal static class Clip
{
    /// <summary>Copies text, retrying briefly when another app holds the clipboard open.</summary>
    public static bool TrySet(string text)
    {
        try
        {
            Clipboard.SetDataObject(text, copy: true, retryTimes: 5, retryDelay: 100);
            return true;
        }
        catch (System.Runtime.InteropServices.ExternalException)
        {
            return false;
        }
    }
}

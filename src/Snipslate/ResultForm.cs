namespace Snipslate;

/// <summary>Small always-on-top popup next to the snip that shows the translation and a Copy button.</summary>
internal sealed class ResultForm : Form
{
    private readonly Rectangle snip;
    private readonly TextBox output;
    private readonly Button copy;
    private readonly Label status;
    private readonly CancellationTokenSource cancellation = new();

    public ResultForm(Rectangle snip, string source)
    {
        this.snip = snip;
        Text = "Snipslate: translation";
        FormBorderStyle = FormBorderStyle.SizableToolWindow;
        StartPosition = FormStartPosition.Manual;
        TopMost = true;
        ShowInTaskbar = false;
        AutoScaleMode = AutoScaleMode.Dpi;
        AutoScaleDimensions = new SizeF(96F, 96F);
        Font = SystemFonts.MessageBoxFont ?? Font;
        MinimumSize = new Size(260, 150);
        ClientSize = new Size(440, 220);

        output = new TextBox
        {
            Multiline = true,
            ReadOnly = true,
            ScrollBars = ScrollBars.Vertical,
            BorderStyle = BorderStyle.None,
            BackColor = SystemColors.Window,
            Dock = DockStyle.Fill,
            Font = new Font("Segoe UI", 11F),
            Text = "Translating…",
            AccessibleName = "Translation",
        };
        copy = new Button { Text = "&Copy", AutoSize = true, Enabled = false };
        status = new Label
        {
            Text = source,
            AutoSize = true,
            ForeColor = SystemColors.GrayText,
            Margin = new Padding(8, 9, 0, 0),
        };
        copy.Click += (_, _) =>
        {
            status.Text = Clip.TrySet(output.Text) ? "Copied to the clipboard" : "The clipboard is busy. Try again.";
        };

        var body = new Panel { Dock = DockStyle.Fill, Padding = new Padding(10), BackColor = SystemColors.Window };
        body.Controls.Add(output);
        var bar = new FlowLayoutPanel { Dock = DockStyle.Bottom, AutoSize = true, Padding = new Padding(6) };
        bar.Controls.Add(copy);
        bar.Controls.Add(status);
        Controls.Add(body);
        Controls.Add(bar);
    }

    /// <summary>Cancelled when the popup closes, so a pending request stops.</summary>
    public CancellationToken Cancellation => cancellation.Token;

    public void ShowResult(string text)
    {
        if (IsDisposed)
        {
            return;
        }

        output.Text = text.ReplaceLineEndings("\r\n");
        copy.Enabled = true;
        copy.Focus();
    }

    public void ShowError(string message)
    {
        if (IsDisposed)
        {
            return;
        }

        output.ForeColor = Color.Firebrick;
        output.Text = message.ReplaceLineEndings("\r\n");
        status.Text = "Translation failed";
        copy.Enabled = false;
    }

    protected override void OnLoad(EventArgs e)
    {
        base.OnLoad(e);

        // Below the snip if it fits on that screen, otherwise above it.
        var area = Screen.FromRectangle(snip).WorkingArea;
        int x = Math.Clamp(snip.Left, area.Left, Math.Max(area.Left, area.Right - Width));
        int y = snip.Bottom + 8 + Height <= area.Bottom
            ? snip.Bottom + 8
            : Math.Max(area.Top, snip.Top - Height - 8);
        Location = new Point(x, y);
    }

    protected override bool ProcessCmdKey(ref Message msg, Keys keyData)
    {
        if (keyData == Keys.Escape)
        {
            Close();
            return true;
        }

        return base.ProcessCmdKey(ref msg, keyData);
    }

    protected override void OnFormClosed(FormClosedEventArgs e)
    {
        cancellation.Cancel();
        base.OnFormClosed(e);
    }

    protected override void Dispose(bool disposing)
    {
        if (disposing)
        {
            cancellation.Dispose();
        }

        base.Dispose(disposing);
    }
}

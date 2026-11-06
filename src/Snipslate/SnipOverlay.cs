using System.Drawing.Imaging;

namespace Snipslate;

/// <summary>
/// Freezes the whole virtual screen, dims it, and lets the user drag a box, like Snipping Tool.
/// The screenshot is taken before the overlay appears, so the overlay never ends up in the image.
/// </summary>
internal sealed class SnipOverlay : Form
{
    private const int WmDpiChanged = 0x02E0;
    private const int MinSize = 4;
    private const string Hint = "Drag to select an area  ·  Esc to cancel";

    private readonly Bitmap screen;
    private readonly Bitmap dimmed;
    private readonly Rectangle hintArea;
    private Point start;
    private Rectangle selection;
    private bool dragging;

    private SnipOverlay(Bitmap screen, Rectangle bounds)
    {
        this.screen = screen;
        dimmed = new Bitmap(screen);
        using (var g = Graphics.FromImage(dimmed))
        using (var shade = new SolidBrush(Color.FromArgb(110, 0, 0, 0)))
        {
            g.FillRectangle(shade, 0, 0, dimmed.Width, dimmed.Height);
        }

        var cursorScreen = Screen.FromPoint(Cursor.Position).Bounds;
        hintArea = new Rectangle(cursorScreen.X - bounds.X, cursorScreen.Y - bounds.Y + 24, cursorScreen.Width, 40);

        Text = "Snipslate: select an area";
        AccessibleName = Text;
        FormBorderStyle = FormBorderStyle.None;
        StartPosition = FormStartPosition.Manual;
        AutoScaleMode = AutoScaleMode.None;
        ShowInTaskbar = false;
        TopMost = true;
        DoubleBuffered = true;
        Cursor = Cursors.Cross;
        Bounds = bounds;
    }

    /// <summary>Returns the selected pixels and where they were on screen, or null if the user cancelled.</summary>
    public static (Bitmap Image, Rectangle ScreenRect)? Pick()
    {
        var bounds = SystemInformation.VirtualScreen;
        using var screen = new Bitmap(bounds.Width, bounds.Height, PixelFormat.Format32bppArgb);
        using (var g = Graphics.FromImage(screen))
        {
            g.CopyFromScreen(bounds.Location, Point.Empty, bounds.Size);
        }

        using var overlay = new SnipOverlay(screen, bounds);
        if (overlay.ShowDialog() != DialogResult.OK)
        {
            return null;
        }

        var area = overlay.selection;
        var image = screen.Clone(area, PixelFormat.Format32bppArgb);
        return (image, new Rectangle(area.X + bounds.X, area.Y + bounds.Y, area.Width, area.Height));
    }

    protected override void OnShown(EventArgs e)
    {
        base.OnShown(e);
        Activate();
    }

    protected override void OnPaint(PaintEventArgs e)
    {
        var g = e.Graphics;
        var whole = new Rectangle(0, 0, screen.Width, screen.Height);
        g.DrawImage(dimmed, whole, whole, GraphicsUnit.Pixel);
        if (selection.Width > 0 && selection.Height > 0)
        {
            g.DrawImage(screen, selection, selection, GraphicsUnit.Pixel);
            using var pen = new Pen(SystemColors.Highlight, 2);
            g.DrawRectangle(pen, selection);
        }
        else
        {
            TextRenderer.DrawText(g, Hint, SystemFonts.MessageBoxFont ?? Font, hintArea, Color.White,
                TextFormatFlags.HorizontalCenter | TextFormatFlags.VerticalCenter);
        }
    }

    protected override void OnMouseDown(MouseEventArgs e)
    {
        if (e.Button == MouseButtons.Right)
        {
            DialogResult = DialogResult.Cancel;
            return;
        }

        if (e.Button == MouseButtons.Left)
        {
            dragging = true;
            start = e.Location;
            SetSelection(Rectangle.Empty);
            Invalidate(hintArea);
        }
    }

    protected override void OnMouseMove(MouseEventArgs e)
    {
        if (dragging)
        {
            var rect = Rectangle.FromLTRB(
                Math.Min(start.X, e.X), Math.Min(start.Y, e.Y), Math.Max(start.X, e.X), Math.Max(start.Y, e.Y));
            SetSelection(Rectangle.Intersect(rect, ClientRectangle));
        }
    }

    protected override void OnMouseUp(MouseEventArgs e)
    {
        if (dragging && e.Button == MouseButtons.Left)
        {
            dragging = false;
            DialogResult = selection.Width >= MinSize && selection.Height >= MinSize
                ? DialogResult.OK
                : DialogResult.Cancel;
        }
    }

    protected override bool ProcessCmdKey(ref Message msg, Keys keyData)
    {
        if (keyData == Keys.Escape)
        {
            DialogResult = DialogResult.Cancel;
            return true;
        }

        return base.ProcessCmdKey(ref msg, keyData);
    }

    protected override void WndProc(ref Message m)
    {
        // The overlay spans monitors with different scaling; it works in physical pixels, so never rescale it.
        if (m.Msg == WmDpiChanged)
        {
            return;
        }

        base.WndProc(ref m);
    }

    protected override void Dispose(bool disposing)
    {
        if (disposing)
        {
            dimmed.Dispose();
        }

        base.Dispose(disposing);
    }

    private void SetSelection(Rectangle next)
    {
        var dirty = Rectangle.Union(selection, next);
        dirty.Inflate(3, 3);
        selection = next;
        Invalidate(dirty);
    }
}

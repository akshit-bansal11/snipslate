namespace Snipslate.Tests;

public class ImagingTests
{
    [Fact]
    public void AllBlackIsBlank()
    {
        using var image = Filled(400, 200, Color.Black);
        Assert.True(Imaging.IsMostlyBlack(image));
    }

    [Fact]
    public void BlackWithVisibleTextAreaIsNotBlank()
    {
        using var image = Filled(400, 200, Color.Black);
        using (var g = Graphics.FromImage(image))
        {
            g.FillRectangle(Brushes.White, 0, 0, 400, 40);
        }

        Assert.False(Imaging.IsMostlyBlack(image));
    }

    [Fact]
    public void DarkThemeIsNotBlank()
    {
        using var image = Filled(400, 200, Color.FromArgb(30, 30, 30));
        Assert.False(Imaging.IsMostlyBlack(image));
    }

    [Fact]
    public void ToPngScalesTheLongEdgeDown()
    {
        using var image = Filled(3000, 1000, Color.White);
        using var decoded = new Bitmap(new MemoryStream(Imaging.ToPng(image)));
        Assert.Equal(1568, decoded.Width);
        Assert.Equal(523, decoded.Height);
    }

    [Fact]
    public void ToPngKeepsSmallImagesAsIs()
    {
        using var image = Filled(300, 120, Color.White);
        using var decoded = new Bitmap(new MemoryStream(Imaging.ToPng(image)));
        Assert.Equal(new Size(300, 120), decoded.Size);
    }

    private static Bitmap Filled(int width, int height, Color color)
    {
        var image = new Bitmap(width, height);
        using var g = Graphics.FromImage(image);
        g.Clear(color);
        return image;
    }
}

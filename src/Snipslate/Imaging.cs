using System.Drawing.Drawing2D;
using System.Drawing.Imaging;

namespace Snipslate;

public static class Imaging
{
    /// <summary>
    /// True when nearly every pixel is black, which is what protected (DRM) video looks like in a capture.
    /// </summary>
    // ponytail: samples a ~100x100 grid; a black terminal with a few words can trip it, so callers offer "Send anyway".
    public static bool IsMostlyBlack(Bitmap image, double threshold = 0.99)
    {
        int stepX = Math.Max(1, image.Width / 100);
        int stepY = Math.Max(1, image.Height / 100);
        int total = 0;
        int dark = 0;
        for (int y = 0; y < image.Height; y += stepY)
        {
            for (int x = 0; x < image.Width; x += stepX)
            {
                var c = image.GetPixel(x, y);
                total++;
                if (c.R + c.G + c.B <= 24)
                {
                    dark++;
                }
            }
        }

        return total > 0 && dark >= total * threshold;
    }

    /// <summary>PNG bytes, scaled down so the long edge is at most <paramref name="maxEdge"/> pixels.</summary>
    public static byte[] ToPng(Bitmap image, int maxEdge = 1568)
    {
        using var stream = new MemoryStream();
        int longEdge = Math.Max(image.Width, image.Height);
        if (longEdge <= maxEdge)
        {
            image.Save(stream, ImageFormat.Png);
            return stream.ToArray();
        }

        double scale = (double)maxEdge / longEdge;
        using var scaled = new Bitmap(
            Math.Max(1, (int)Math.Round(image.Width * scale)),
            Math.Max(1, (int)Math.Round(image.Height * scale)));
        using (var g = Graphics.FromImage(scaled))
        {
            g.InterpolationMode = InterpolationMode.HighQualityBicubic;
            g.DrawImage(image, 0, 0, scaled.Width, scaled.Height);
        }

        scaled.Save(stream, ImageFormat.Png);
        return stream.ToArray();
    }
}

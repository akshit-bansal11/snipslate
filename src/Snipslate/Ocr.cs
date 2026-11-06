using Windows.Graphics.Imaging;
using Windows.Media.Ocr;

namespace Snipslate;

/// <summary>Windows' built-in text recognition. Runs offline, using the user's installed language packs.</summary>
internal static class Ocr
{
    public static async Task<string> ReadTextAsync(Bitmap image)
    {
        var engine = OcrEngine.TryCreateFromUserProfileLanguages()
            ?? throw new InvalidOperationException(
                "Windows has no text-recognition language installed. Add a language in Settings > Time & language > Language & region.");

        var png = Imaging.ToPng(image, (int)OcrEngine.MaxImageDimension);
        using var stream = new MemoryStream(png);
        using var winrtStream = stream.AsRandomAccessStream();
        var decoder = await BitmapDecoder.CreateAsync(winrtStream);
        using var bitmap = await decoder.GetSoftwareBitmapAsync();
        var result = await engine.RecognizeAsync(bitmap);
        return string.Join(Environment.NewLine, result.Lines.Select(line => line.Text)).Trim();
    }
}

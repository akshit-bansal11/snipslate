namespace Snipslate.Tests;

public class HotkeyTests
{
    [Theory]
    [InlineData("Ctrl+Alt+T", Hotkey.Control | Hotkey.Alt, Keys.T)]
    [InlineData(" ctrl + shift + 5 ", Hotkey.Control | Hotkey.Shift, Keys.D5)]
    [InlineData("Win+F1", Hotkey.Win, Keys.F1)]
    [InlineData("Control+Alt+PrintScreen", Hotkey.Control | Hotkey.Alt, Keys.PrintScreen)]
    public void Parses(string text, uint modifiers, Keys key)
    {
        Assert.True(Hotkey.TryParse(text, out var hotkey));
        Assert.Equal(new Hotkey(modifiers, key), hotkey);
    }

    [Theory]
    [InlineData("")]
    [InlineData("T")]
    [InlineData("Ctrl+")]
    [InlineData("Ctrl+Ctrl+T")]
    [InlineData("Ctrl+Alt+Shift")]
    [InlineData("Hyper+T")]
    [InlineData("Ctrl+65")]
    [InlineData("Ctrl+NotAKey")]
    [InlineData("Ctrl+A, B")]
    public void Rejects(string text) => Assert.False(Hotkey.TryParse(text, out _));

    [Fact]
    public void FormatsInCanonicalOrder()
    {
        Assert.True(Hotkey.TryParse("shift+ctrl+7", out var hotkey));
        Assert.Equal("Ctrl+Shift+7", hotkey.ToString());
        Assert.True(Hotkey.TryParse(hotkey.ToString(), out var again));
        Assert.Equal(hotkey, again);
    }
}

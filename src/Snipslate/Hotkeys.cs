using System.Runtime.InteropServices;

namespace Snipslate;

/// <summary>A global hotkey such as Ctrl+Alt+T: RegisterHotKey modifier flags plus one key.</summary>
public readonly record struct Hotkey(uint Modifiers, Keys Key)
{
    public const uint Alt = 0x1;
    public const uint Control = 0x2;
    public const uint Shift = 0x4;
    public const uint Win = 0x8;

    public static bool TryParse(string? text, out Hotkey hotkey)
    {
        hotkey = default;
        if (string.IsNullOrWhiteSpace(text))
        {
            return false;
        }

        var parts = text.Split('+', StringSplitOptions.TrimEntries | StringSplitOptions.RemoveEmptyEntries);
        if (parts.Length < 2)
        {
            return false;
        }

        uint modifiers = 0;
        foreach (var part in parts[..^1])
        {
            uint flag = part.ToUpperInvariant() switch
            {
                "CTRL" or "CONTROL" => Control,
                "ALT" => Alt,
                "SHIFT" => Shift,
                "WIN" or "WINDOWS" => Win,
                _ => 0,
            };
            if (flag == 0 || (modifiers & flag) != 0)
            {
                return false;
            }

            modifiers |= flag;
        }

        var keyName = parts[^1];
        if (int.TryParse(keyName, out var number))
        {
            if (keyName.Length != 1)
            {
                return false;
            }

            keyName = "D" + number;
        }

        if (!Enum.TryParse(keyName, ignoreCase: true, out Keys key) || !IsPlainKey(key))
        {
            return false;
        }

        hotkey = new Hotkey(modifiers, key);
        return true;
    }

    public override string ToString()
    {
        var parts = new List<string>(5);
        if ((Modifiers & Control) != 0)
        {
            parts.Add("Ctrl");
        }

        if ((Modifiers & Alt) != 0)
        {
            parts.Add("Alt");
        }

        if ((Modifiers & Shift) != 0)
        {
            parts.Add("Shift");
        }

        if ((Modifiers & Win) != 0)
        {
            parts.Add("Win");
        }

        parts.Add(Key is >= Keys.D0 and <= Keys.D9 ? ((int)(Key - Keys.D0)).ToString() : Key.ToString());
        return string.Join('+', parts);
    }

    private static bool IsPlainKey(Keys key) =>
        Enum.IsDefined(key)
        && (key & ~Keys.KeyCode) == 0
        && key is not (Keys.None or Keys.ControlKey or Keys.LControlKey or Keys.RControlKey
            or Keys.ShiftKey or Keys.LShiftKey or Keys.RShiftKey
            or Keys.Menu or Keys.LMenu or Keys.RMenu or Keys.LWin or Keys.RWin);
}

/// <summary>Hidden window that owns the registered hotkeys and raises <see cref="Pressed"/>.</summary>
internal sealed class HotkeyWindow : NativeWindow, IDisposable
{
    private const int WmHotkey = 0x0312;
    private const uint ModNoRepeat = 0x4000;
    private readonly List<int> registered = [];

    public HotkeyWindow() => CreateHandle(new CreateParams());

    public event Action<int>? Pressed;

    public bool Register(int id, Hotkey hotkey)
    {
        if (!RegisterHotKey(Handle, id, hotkey.Modifiers | ModNoRepeat, (uint)hotkey.Key))
        {
            return false;
        }

        registered.Add(id);
        return true;
    }

    public void UnregisterAll()
    {
        foreach (var id in registered)
        {
            UnregisterHotKey(Handle, id);
        }

        registered.Clear();
    }

    public void Dispose()
    {
        UnregisterAll();
        DestroyHandle();
    }

    protected override void WndProc(ref Message m)
    {
        if (m.Msg == WmHotkey)
        {
            Pressed?.Invoke((int)m.WParam);
        }

        base.WndProc(ref m);
    }

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool RegisterHotKey(IntPtr hWnd, int id, uint fsModifiers, uint vk);

    [DllImport("user32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    private static extern bool UnregisterHotKey(IntPtr hWnd, int id);
}

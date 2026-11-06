namespace Snipslate;

internal static class Program
{
    [STAThread]
    private static void Main()
    {
        ApplicationConfiguration.Initialize();

        using var mutex = new Mutex(true, @"Local\Snipslate-5taorn0rz0", out var firstInstance);
        if (!firstInstance)
        {
            MessageBox.Show("Snipslate is already running. Look for its icon in the system tray.", "Snipslate");
            return;
        }

        Application.SetUnhandledExceptionMode(UnhandledExceptionMode.CatchException);
        Application.ThreadException += (_, e) =>
            MessageBox.Show(e.Exception.Message, "Snipslate error", MessageBoxButtons.OK, MessageBoxIcon.Error);
        Application.Run(new TrayApp());
    }
}

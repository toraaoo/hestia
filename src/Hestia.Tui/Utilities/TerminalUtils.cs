namespace Hestia.Tui.Utilities;

internal static class TerminalUtils
{
    public const int HeaderH   = 7;
    public const int JreH      = 7;
    public const int LeftMinW  = 44;
    public const int LeftMaxW  = 64;
    public const int RightMinW = 60;
    public const int MinWidth  = 100;
    public const int MinHeight = 24;

    public static bool TooSmall() =>
        Console.WindowWidth < MinWidth || Console.WindowHeight < MinHeight;

    public static int ComputeLeftWidth(int totalW)
    {
        var target = (int)Math.Round(totalW * 0.45);
        target = Math.Clamp(target, LeftMinW, LeftMaxW);

        var maxLeft = totalW - RightMinW;
        if (maxLeft >= LeftMinW)
            return Math.Min(target, maxLeft);

        return Math.Max(20, totalW / 2);
    }
}

namespace Hestia.Tui.Utils.Extensions;

public static class StringExtensions
{
    public static string Truncate(this string str, int maxLength) =>
        str.Length <= maxLength ? str : string.Concat(str.AsSpan(0, maxLength - 1), "…");
}
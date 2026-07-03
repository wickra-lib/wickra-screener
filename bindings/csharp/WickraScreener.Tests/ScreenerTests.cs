using System.Text.Json;
using Wickra.Screener;
using Xunit;

namespace WickraScreener.Tests;

public class ScreenerTests
{
    private const string Spec =
        "{\"universe\":[\"AAA\",\"BBB\"],\"condition\":{\"type\":\"cmp\"," +
        "\"left\":{\"kind\":\"price\",\"field\":\"close\"},\"op\":\"gt\"," +
        "\"right\":{\"kind\":\"const\",\"value\":10.0}}}";

    private static object Candle(double close) =>
        new { time = 1, open = close, high = close, low = close, close, volume = 1.0 };

    [Fact]
    public void Version_IsNonEmpty()
    {
        Assert.False(string.IsNullOrEmpty(Screener.Version()));
    }

    [Fact]
    public void Scan_ReturnsMatchingSymbol()
    {
        using var screener = new Screener(Spec);
        string cmd = JsonSerializer.Serialize(new
        {
            cmd = "scan",
            data = new Dictionary<string, object[]>
            {
                ["AAA"] = [Candle(5)],
                ["BBB"] = [Candle(15)],
            },
        });

        string raw = screener.Command(cmd);
        using JsonDocument report = JsonDocument.Parse(raw);

        Assert.Equal(2, report.RootElement.GetProperty("scanned").GetInt32());
        JsonElement matches = report.RootElement.GetProperty("matches");
        Assert.Equal(1, matches.GetArrayLength());
        Assert.Equal("BBB", matches[0].GetProperty("symbol").GetString());
    }

    [Fact]
    public void InvalidSpec_Throws()
    {
        Assert.Throws<ArgumentException>(() => new Screener("not json"));
    }

    [Fact]
    public void UnknownCommand_IsInBandError()
    {
        using var screener = new Screener(Spec);
        // An unknown command is not a hard error: the ABI returns a length and the
        // error surfaces in-band as {"ok":false,...} JSON.
        string raw = screener.Command("{\"cmd\":\"nope\"}");
        Assert.Contains("\"ok\":false", raw);
    }
}

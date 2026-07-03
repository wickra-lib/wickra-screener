// A runnable .NET example: scan a small universe through the binding.
//
//   cargo build --release -p wickra-screener-c
//   dotnet run --project examples/csharp/Scan

using System.Text.Json;
using Wickra.Screener;

const string spec =
    "{\"universe\":[\"AAA\",\"BBB\"],\"condition\":{\"type\":\"cmp\"," +
    "\"left\":{\"kind\":\"price\",\"field\":\"close\"},\"op\":\"gt\"," +
    "\"right\":{\"kind\":\"const\",\"value\":10.0}}}";

static object Candle(double close) =>
    new { time = 1, open = close, high = close, low = close, close, volume = 1.0 };

using var screener = new Screener(spec);

string cmd = JsonSerializer.Serialize(new
{
    cmd = "scan",
    data = new Dictionary<string, object[]>
    {
        ["AAA"] = [Candle(5)],
        ["BBB"] = [Candle(15)],
    },
});

string response = screener.Command(cmd);
using JsonDocument report = JsonDocument.Parse(response);

Console.WriteLine($"wickra-screener {Screener.Version()}");
Console.WriteLine(response);
foreach (JsonElement match in report.RootElement.GetProperty("matches").EnumerateArray())
{
    Console.WriteLine($"  matched: {match.GetProperty("symbol").GetString()}");
}

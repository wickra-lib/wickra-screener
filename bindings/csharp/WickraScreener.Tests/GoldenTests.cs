using System.Text.Json;
using Wickra.Screener;
using Xunit;

namespace WickraScreener.Tests;

// Cross-language golden parity: build the screener from each committed
// golden/specs/*.json, run a scan over the shared golden/data.json, and assert
// the response equals golden/expected/<spec>.json byte-for-byte. The binding
// returns the core's compact command_json string verbatim, so byte equality is
// the exact cross-language parity check. The fixtures arrive in a later phase;
// until then the test skips cleanly.
public class GoldenTests
{
    private static string? FindGolden()
    {
        string? dir = AppContext.BaseDirectory;
        for (int i = 0; i < 10 && dir is not null; i++)
        {
            string g = Path.Combine(dir, "golden");
            if (Directory.Exists(Path.Combine(g, "specs")))
            {
                return g;
            }
            dir = Path.GetDirectoryName(dir);
        }
        return null;
    }

    [Fact]
    public void GoldenScans_AreByteIdentical()
    {
        string? golden = FindGolden();
        if (golden is null)
        {
            return; // golden fixtures not present yet
        }

        string dataset = File.ReadAllText(Path.Combine(golden, "data.json"));
        using JsonDocument data = JsonDocument.Parse(dataset);

        foreach (string specPath in Directory.GetFiles(Path.Combine(golden!, "specs"), "*.json"))
        {
            string spec = File.ReadAllText(specPath);
            string name = Path.GetFileName(specPath);
            string expected = File.ReadAllText(Path.Combine(golden!, "expected", name)).TrimEnd();

            using var screener = new Screener(spec);
            string cmd = JsonSerializer.Serialize(new
            {
                cmd = "scan",
                data = data.RootElement,
            });
            string raw = screener.Command(cmd);
            Assert.Equal(expected, raw.TrimEnd());
        }
    }
}

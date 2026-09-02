using System.Text.Json;
using Wickra.Screener;
using Xunit;

namespace WickraScreener.Tests;

// Streaming equals batch, driven through the JSON command boundary.
//
// screener-core proves this in Rust
// (crates/screener-core/tests/streaming_eq_batch.rs), but that says nothing about
// the boundary each language actually crosses. A binding reaches the core through
// Command, and the golden corpus only ever sends {"cmd":"scan"} -- so feed and
// evaluate were exercised in no language at all. That is how the C ABI shipped a
// command that ran twice under the two-call idiom: scan is a pure function of its
// payload, so it looked correct while every feed applied its candle twice.
//
// Batch: one scan over the whole dataset.
// Streaming: feed each candle of each symbol in turn, then evaluate.
// Both return the core's compact JSON verbatim, so byte equality is the check.
//
// Only the candle-only specs are used. A feeds_* spec needs side feeds the
// streaming envelope carries per bar, and derived_breadth needs the market panel,
// which a streaming screener cannot derive: it sees one symbol's bar at a time and
// cannot know which other symbols print at that timestamp. That is a documented
// limitation (docs/CROSS_SECTION.md), not something to hide here.
public class StreamingTests
{
    private static readonly string[] Specs =
    {
        "momentum",
        "mean_reversion",
        "cross_section_rank",
        "breadth",
        "crossover",
        "compound",
    };

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

    private static void FeedAll(Screener screener, JsonDocument data)
    {
        List<string> symbols = new();
        foreach (JsonProperty symbol in data.RootElement.EnumerateObject())
        {
            symbols.Add(symbol.Name);
        }
        symbols.Sort(StringComparer.Ordinal);

        foreach (string symbol in symbols)
        {
            foreach (JsonElement candle in data.RootElement.GetProperty(symbol).EnumerateArray())
            {
                string envelope =
                    $"{{\"cmd\":\"feed\",\"symbol\":{JsonSerializer.Serialize(symbol)}," +
                    $"\"candle\":{candle.GetRawText()}}}";
                screener.Command(envelope);
            }
        }
    }

    [Fact]
    public void Streaming_EqualsBatch()
    {
        string? golden = FindGolden();
        if (golden is null)
        {
            return; // golden fixtures not present yet
        }

        string dataText = File.ReadAllText(Path.Combine(golden, "data.json"));
        using JsonDocument data = JsonDocument.Parse(dataText);

        int compared = 0;
        foreach (string name in Specs)
        {
            string specPath = Path.Combine(golden, "specs", name + ".json");
            Assert.True(File.Exists(specPath), $"spec {name} is missing from the corpus");
            string spec = File.ReadAllText(specPath);

            string batch;
            using (Screener batchScreener = new(spec))
            {
                batch = batchScreener
                    .Command($"{{\"cmd\":\"scan\",\"data\":{dataText}}}")
                    .Trim();
            }

            string streamed;
            using (Screener streaming = new(spec))
            {
                FeedAll(streaming, data);
                streamed = streaming.Command("{\"cmd\":\"evaluate\"}").Trim();
            }

            Assert.True(streamed == batch, $"streaming != batch for spec {name}");
            compared++;
        }

        // A loop that compared nothing passes; say how many it actually did.
        Assert.Equal(Specs.Length, compared);
    }

    [Fact]
    public void Reset_ReturnsToThePreFeedState()
    {
        string? golden = FindGolden();
        if (golden is null)
        {
            return; // golden fixtures not present yet
        }

        string dataText = File.ReadAllText(Path.Combine(golden, "data.json"));
        using JsonDocument data = JsonDocument.Parse(dataText);
        string spec = File.ReadAllText(Path.Combine(golden, "specs", "momentum.json"));

        using Screener screener = new(spec);
        string empty = screener.Command("{\"cmd\":\"evaluate\"}");

        FeedAll(screener, data);
        Assert.True(
            screener.Command("{\"cmd\":\"evaluate\"}") != empty,
            "feeding the whole universe changed nothing");

        screener.Command("{\"cmd\":\"reset\"}");
        Assert.True(
            screener.Command("{\"cmd\":\"evaluate\"}") == empty,
            "reset did not return the screener to its pre-feed state");
    }
}

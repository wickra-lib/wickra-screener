# Wickra Screener examples — C#

Runnable C# examples for the [Wickra Screener C# binding](../../bindings/csharp). The binding consumes the C ABI
library through P/Invoke, so build it once before running anything:

```bash
cargo build -p wickra-screener-c --release
```

## Run

As the CI examples job runs it, from the repository root:

```bash
dotnet run --project examples/csharp/Scan
```

## The examples

| Example | What it does |
|---------|--------------|
| `Scan/Program.cs` | A runnable .NET example: scan a small universe through the binding. |

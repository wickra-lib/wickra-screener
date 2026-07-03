using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Wickra.Screener;

/// <summary>
/// Locates the native <c>wickra_screener</c> library across the dev, CI and
/// packaged layouts. Registered as the assembly's P/Invoke resolver at module
/// load. Every candidate is validated by a sentinel export check so a wrong or
/// stale library is rejected rather than silently loaded.
/// </summary>
internal static class DllResolver
{
    private const string Sentinel = "wickra_screener_version";

    // Registering the P/Invoke resolver at assembly load is exactly the
    // module-initializer use case; CA2255's "advanced scenarios only" advice does
    // not apply here.
#pragma warning disable CA2255
    [ModuleInitializer]
    internal static void Register() =>
        NativeLibrary.SetDllImportResolver(typeof(DllResolver).Assembly, Resolve);
#pragma warning restore CA2255

    private static IntPtr Resolve(string libraryName, Assembly assembly, DllImportSearchPath? searchPath)
    {
        if (libraryName != Native.Lib)
        {
            return IntPtr.Zero;
        }

        // 1) Default resolution: app dir, PATH, and runtimes/<rid>/native in a package.
        if (NativeLibrary.TryLoad(Native.Lib, assembly, searchPath, out IntPtr handle) && HasSentinel(handle))
        {
            return handle;
        }

        // 2) Dev/CI fallback: walk up from the assembly directory to a Cargo
        //    target/{release,debug} directory holding the built library.
        foreach (string candidate in Candidates(assembly))
        {
            if (NativeLibrary.TryLoad(candidate, out handle) && HasSentinel(handle))
            {
                return handle;
            }
        }

        return IntPtr.Zero;
    }

    private static bool HasSentinel(IntPtr handle) =>
        NativeLibrary.TryGetExport(handle, Sentinel, out _);

    private static IEnumerable<string> Candidates(Assembly assembly)
    {
        string fileName = LibraryFileName();
        string? dir = Path.GetDirectoryName(assembly.Location);
        while (!string.IsNullOrEmpty(dir))
        {
            foreach (string profile in new[] { "release", "debug" })
            {
                yield return Path.Combine(dir, "target", profile, fileName);
            }
            dir = Path.GetDirectoryName(dir);
        }
    }

    private static string LibraryFileName()
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            return $"{Native.Lib}.dll";
        }
        if (RuntimeInformation.IsOSPlatform(OSPlatform.OSX))
        {
            return $"lib{Native.Lib}.dylib";
        }
        return $"lib{Native.Lib}.so";
    }
}

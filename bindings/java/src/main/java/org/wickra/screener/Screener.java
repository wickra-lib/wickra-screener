package org.wickra.screener;

import java.lang.foreign.Arena;
import java.lang.foreign.MemorySegment;

/**
 * A screener instance driven by JSON commands, over the Wickra C ABI
 * (FFM/Panama). Build one from a spec JSON, drive it with command JSON and read
 * back the response JSON — the same protocol as the CLI and every other binding.
 */
public final class Screener implements AutoCloseable {
    private MemorySegment handle;

    /** Build a screener from a spec JSON string. */
    public Screener(String specJson) {
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment spec = arena.allocateFrom(specJson);
            MemorySegment created = (MemorySegment) Native.NEW.invokeExact(spec);
            if (created.address() == 0) {
                throw new IllegalArgumentException("wickra-screener: invalid spec");
            }
            this.handle = created;
        } catch (RuntimeException | Error e) {
            throw e;
        } catch (Throwable t) {
            throw new RuntimeException(t);
        }
    }

    /**
     * Apply a command JSON and return the response JSON. Uses the C ABI's
     * length-out protocol: a first call learns the length, then the response is
     * read into a caller-owned buffer. Domain errors (a bad spec, an unknown
     * command) come back in-band as {@code {"ok":false,...}} JSON, not as an
     * exception.
     */
    public String command(String cmdJson) {
        if (handle == null) {
            throw new IllegalStateException("screener is closed");
        }
        try (Arena arena = Arena.ofConfined()) {
            MemorySegment cmd = arena.allocateFrom(cmdJson);
            int len = (int) Native.COMMAND.invokeExact(handle, cmd, MemorySegment.NULL, 0L);
            if (len < 0) {
                throw new IllegalStateException("wickra-screener: command failed (code " + len + ")");
            }
            MemorySegment buf = arena.allocate(len + 1L);
            int written = (int) Native.COMMAND.invokeExact(handle, cmd, buf, (long) (len + 1));
            return buf.getString(0);
        } catch (RuntimeException | Error e) {
            throw e;
        } catch (Throwable t) {
            throw new RuntimeException(t);
        }
    }

    /** The library version. */
    public static String version() {
        try {
            MemorySegment ptr = (MemorySegment) Native.VERSION.invokeExact();
            return ptr.reinterpret(Long.MAX_VALUE).getString(0);
        } catch (Throwable t) {
            throw new RuntimeException(t);
        }
    }

    /** Free the native screener handle. */
    @Override
    public void close() {
        if (handle != null) {
            try {
                Native.FREE.invokeExact(handle);
            } catch (Throwable t) {
                throw new RuntimeException(t);
            }
            handle = null;
        }
    }
}

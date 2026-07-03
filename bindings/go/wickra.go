// Package wickra provides idiomatic Go bindings for wickra-screener over its C
// ABI hub: build a Screener from a spec JSON, drive it with command JSON and read
// back the response JSON — the same protocol as the CLI and every other binding.
//
// The binding links the prebuilt C ABI library, staged per platform under
// ./lib/<goos>_<goarch>/, with the header vendored under ./include.
package wickra

/*
#cgo CFLAGS: -I${SRCDIR}/include
#cgo linux,amd64 LDFLAGS: -L${SRCDIR}/lib/linux_amd64 -lwickra_screener -Wl,-rpath,${SRCDIR}/lib/linux_amd64
#cgo linux,arm64 LDFLAGS: -L${SRCDIR}/lib/linux_arm64 -lwickra_screener -Wl,-rpath,${SRCDIR}/lib/linux_arm64
#cgo darwin,amd64 LDFLAGS: -L${SRCDIR}/lib/darwin_amd64 -lwickra_screener -Wl,-rpath,${SRCDIR}/lib/darwin_amd64
#cgo darwin,arm64 LDFLAGS: -L${SRCDIR}/lib/darwin_arm64 -lwickra_screener -Wl,-rpath,${SRCDIR}/lib/darwin_arm64
#cgo windows,amd64 LDFLAGS: -L${SRCDIR}/lib/windows_amd64 -l:wickra_screener.dll
#cgo windows,arm64 LDFLAGS: -L${SRCDIR}/lib/windows_arm64 -l:wickra_screener.dll
#include <stdlib.h>
#include "wickra_screener.h"
*/
import "C"

import (
	"fmt"
	"runtime"
	"unsafe"
)

// Screener is a screener instance driven by JSON commands.
type Screener struct {
	handle *C.WickraScreener
}

// New builds a screener from a spec JSON string. Call Close when done (a
// finalizer also frees it, but explicit Close is preferred).
func New(specJSON string) (*Screener, error) {
	cspec := C.CString(specJSON)
	defer C.free(unsafe.Pointer(cspec))

	handle := C.wickra_screener_new(cspec)
	if handle == nil {
		return nil, fmt.Errorf("wickra-screener: invalid spec")
	}
	s := &Screener{handle: handle}
	runtime.SetFinalizer(s, (*Screener).Close)
	return s, nil
}

// Command applies a command JSON and returns the response JSON. It uses the C
// ABI's length-out protocol: a first call learns the length, then the response
// is read into a caller-owned buffer.
func (s *Screener) Command(cmdJSON string) (string, error) {
	ccmd := C.CString(cmdJSON)
	defer C.free(unsafe.Pointer(ccmd))

	n := C.wickra_screener_command(s.handle, ccmd, nil, 0)
	if n < 0 {
		return "", fmt.Errorf("wickra-screener: command failed (code %d)", int(n))
	}
	buf := make([]byte, int(n)+1)
	C.wickra_screener_command(
		s.handle,
		ccmd,
		(*C.char)(unsafe.Pointer(&buf[0])),
		C.size_t(len(buf)),
	)
	return string(buf[:n]), nil
}

// Close frees the screener handle. Safe to call more than once.
func (s *Screener) Close() {
	if s.handle != nil {
		C.wickra_screener_free(s.handle)
		s.handle = nil
	}
	runtime.SetFinalizer(s, nil)
}

// Version returns the library version.
func Version() string {
	return C.GoString(C.wickra_screener_version())
}

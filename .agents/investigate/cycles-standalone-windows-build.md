# Cycles Standalone: can it be built and run on this machine, and at what cost

Recorded 2026-10-07. Question answered: what does it actually take to get a
`cycles` binary on this Windows box, and is the "day-scale obstacle" from
`offline-renderer-backend.md` real?

**Short answer: no, that obstacle is not real, and the earlier claim is retracted.**
The build is a submodule fetch and a CMake configure. This file records the
correction so the next reader does not inherit it.

## What was wrong with the earlier claim

`offline-renderer-backend.md` said:

> On Windows, FreeCAD's render workbench documentation describes the build as
> "tricky and time-consuming", since OIIO, Boost, GLEW, PugiXML and possibly OSL
> and the CUDA or OptiX software development kits must be assembled by hand. We
> develop on Windows with MSVC, so this is a genuine day-scale obstacle.

That reasoning treats the standalone repository as if it had no dependency
automation. It does. The claim was sourced from a third-party wiki describing a
build from an era when dependencies were checked out from a Subversion tree, and
was carried forward without checking whether the current repository still worked
that way. It does not.

`make.bat update` invokes `python src/cmake/make_update.py`, whose entire job is
`git submodule` operations against `lib/<platform>_<arch>`. `.gitmodules` points at
`projects.blender.org/blender/lib-windows_x64.git`. There is nothing to assemble by
hand. The old README text quoted in search results still says
`https://svn.blender.org/svnroot/bf-blender/trunk/lib/`; that is the Subversion
layout, and it is gone.

## The precompiled dependency set

Listed from the `lib-windows_x64` repository root on 2026-10-07. The set covers
everything the current `BUILDING.md` calls required or optional:

`OpenImageIO`, `tbb`, `osl`, `embree`, `openimagedenoise`, `openvdb`,
`opensubdiv`, `opencolorio`, `alembic`, `openexr`, `pugixml`, `sdl`, `MaterialX`,
`usd`, `python`, `llvm`, `vulkan`, and the image and compression libraries.

Two consequences:

- **OpenImageDenoise is present.** Denoising is available without extra work,
  which is a real advantage over the alternatives in the previous survey.
- **`usd` is present**, last updated 2026-09-30. It was not needed in the end; see
  the note at the end of this file.

## The build, on this machine

`BUILDING.md` gives the standalone path as `make update` then `make`. On Windows
that is `make.bat update` then `make.bat`, and `make.bat` with no argument runs
`release`.

Toolchain present, measured 2026-10-07:

| Tool | Version |
| --- | --- |
| Visual Studio Build Tools | 2026, 18.4.11620.152 |
| CMake | 4.2.0 |
| Python | 3.12.8 |
| Git | 2.39.2.windows.1 |
| git-lfs | 3.3.0 |
| Logical processors | 20 |

`BUILDING.md` lists Git, Git LFS, Python 3 and CMake as the prerequisites, and
`make_update.py` confirms all four are used: Git for submodules, LFS because the
libraries are stored through it, Python to drive the script, CMake to configure.
All four are present. The only unlisted requirement is a C++17 compiler, which the
Build Tools install supplies.

The build is therefore:

```
git clone https://projects.blender.org/blender/cycles.git
cd cycles
make.bat update
make.bat
```

The result is `./install/cycles`. There is no dependency assembly step, no manual
SDK hunting, and no hand-configured include or library path.

## Two things that will actually cost time

Neither is the day-scale assembly, but both are real.

**CMake 4.2 against `cmake_minimum_required(VERSION 3.10)`.** The root
`CMakeLists.txt` declares a 3.10 minimum. CMake 4.x removed compatibility with
declarations below 3.5, so 3.10 is *not* in the removed range, but the wider
ecosystem of the precompiled libraries may declare older minimums. This is the most
likely first failure and it is a five-minute check: run the configure step and read
the error. If it fails, the fallback is to install a 3.x CMake rather than to patch
upstream.

**CUDA and OptiX binaries are not built by default.** `CMakeLists.txt` sets
`WITH_CYCLES_CUDA_BINARIES` to `OFF` and `WITH_CYCLES_HIP_BINARIES` to `OFF`. The
device backends compile in either way, and `WITH_CYCLES_RUNTIME_OPTIX_ROOT_DIR`
allows compiling OptiX kernels at runtime instead. So a CPU-only build is
unaffected, and a GPU build needs either a one-off kernel compile or the runtime
OptiX path configured. Worth resolving before committing to GPU as the default
device, not before the first build.

Note also that `WITH_CYCLES_USD` defaults to `ON`, and `BUILDING.md` warns that
building against a USD repository disables OSL and NanoVDB. Since USD is not being
used, the build should pass `-DWITH_CYCLES_USD=OFF` to keep OSL available and keep
the build small.

## Outcome: it built, and it renders

Built on this machine on 2026-10-08. `install/cycles.exe`, 7.4 MB, reports
version `5.3.0`. It renders: `examples/scene_monkey.xml` at 16 samples on CPU
completed in 0.9 s and wrote a correct image, Suzanne mesh, checkerboard floor,
glass refraction and shadow ray all present.

```
cycles.exe --device CPU --samples 16 --background --quiet \
           --output out.png examples/scene_monkey.xml
```

`--list-devices` reports both a CPU and an `NVIDIA GeForce RTX 3060 Laptop GPU`
under CUDA. The OptiX backend is disabled because no OptiX SDK is installed, so
GPU work would go through the CUDA backend. That path is untested.

The estimate in this file was therefore roughly right in kind and understated in
time: the obstacle is compile time, plus dependency download, neither of which
blocked us.

## Two traps worth keeping

**`WITH_CYCLES_USD=OFF` does not disable USD.** `src/cmake/external_libs.cmake`
opens with

```cmake
if(WITH_CYCLES_HYDRA_RENDER_DELEGATE OR WITH_CYCLES_USD)
  set(WITH_USD ON)
endif()
```

and `WITH_CYCLES_HYDRA_RENDER_DELEGATE` defaults to `ON`. So the USD switch is
overridden and the `WITH_USD` macro lands in every generated project file, which
is what `src/app/cycles_standalone.cpp` tests to choose the USD reader over the
XML one. The cache will honestly report `WITH_CYCLES_USD:BOOL=OFF` while the
macro is still defined, so the cache is not a safe place to check. Read the
`PreprocessorDefinitions` in the generated `.vcxproj` instead. Passing
`-DWITH_CYCLES_HYDRA_RENDER_DELEGATE=OFF` alongside it does work, and the log
then stops printing `Found USD`.

**Git LFS is not pulled by the submodule step.** `make_update.py` runs
`GIT_LFS_SKIP_SMUDGE=1 git submodule update`, which fetches Git objects only and
then shells out to `git lfs pull`. If that final step is interrupted, the
submodule looks complete — 218 MB of objects, ~6.5 GB of working tree, thousands
of files present — while the binaries are still 130-byte pointer files. The
symptom surfaces only at build time, as an MSBuild step that reports the useless
`unknown error` and `MSB8066`.

To check whether LFS really landed, read the size of a specific file rather than
totalling the directory:

```
Get-Item lib/windows_x64/osl/bin/oslc.exe | Select-Object Length
```

A real `oslc.exe` is 38400 bytes, matching the `size` line in the pointer it
replaces. 130 means the pointer is still there. `Get-Content` will happily print
the three lines of a pointer file, which makes it useless as a test.

## The configuration that was built

```
cmake -B build -DWITH_CYCLES_USD=OFF -DWITH_CYCLES_HYDRA_RENDER_DELEGATE=OFF \
      -DWITH_CYCLES_OSL=OFF -DWITH_CYCLES_ALEMBIC=OFF \
      -DWITH_CYCLES_OPENSUBDIV=OFF -DWITH_CYCLES_OPENVDB=OFF \
      -DWITH_CYCLES_NANOVDB=OFF -DWITH_CYCLES_DEVICE_HIP=OFF
cmake --build build --target install --config Release --parallel 20
```

Verified in the generated Release `PreprocessorDefinitions` afterwards:
`WITH_USD`, `WITH_OSL`, `WITH_NANOVDB`, `WITH_OPENVDB`, `WITH_OPENSUBDIV`,
`WITH_PYTHON` and `WITH_HIP` all absent; `WITH_EMBREE`, `WITH_OPENIMAGEDENOISE`
and `WITH_OCIO` retained. Embree is the BVH and is not optional for CPU render
performance. PugiXML must never be disabled — it is the XML parser the entire
scene format depends on.

Turning OSL off skips the shader compilation step, which is the only build step
that shells out to `oslc.exe`. With OSL off we use the SVM node graph, which is
where `principled_bsdf` lives, so nothing we planned on needed it.

## A note on network behaviour

Cloning through `git clone --progress` stalled for 17 minutes with an empty
object store, while `git ls-remote` over the same URL returned immediately. The
difference was the progress output being buffered in a background pipe. Using
`--filter=blob:none --no-checkout` and letting the checkout happen separately
finished the clone in 47 s.

This machine has `HTTP_PROXY` and `HTTPS_PROXY` set to `127.0.0.1:10793`. The
bandwidth observed against `projects.blender.org` was around 190–450 KiB/s, and
the full dependency set is several gigabytes, so the download was the longest
single step of the whole exercise — several hours. That is the real cost, and it
is worth knowing before starting on a machine with a slow link.

## Unverified

- **The CUDA path.** A device is listed but never exercised.
- **Multi-configuration builds.** Only `Release` was built. The generated project
  files still contain Debug, MinSizeRel and RelWithDebInfo entries.
- **No scene from our own compiler has been rendered.** The probes here use
  hand-written XML in the shape our emitter would produce, not output from
  `crates/scene`.
- **`exposure.rs` on our side.** See
  [cycles-standalone-scene-format.md](cycles-standalone-scene-format.md); the
  Cycles side is measured, ours is not.

## Note on the USD decision

The USD claim in `offline-renderer-backend.md` was flagged as the first thing to
verify and the pivot for the whole recommendation. It is now resolved, and
resolved in the negative for our purposes: the standalone app does read USD, under
`WITH_USD`, via `HD_CYCLES_NS::HdCyclesFileReader::read`, and USD is present in the
precompiled dependency set.

It is still the wrong choice here, for the reasons given: it duplicates what
`StandardScene` already models, each renderer has its own specialisms regardless,
writing a per-renderer emitter is a small and bounded job, and a USD toolchain
would have to be driven from Rust to get any benefit. The claim was true and the
conclusion it supported was wrong. `cycles-standalone-scene-format.md` records what
the XML format actually accepts.

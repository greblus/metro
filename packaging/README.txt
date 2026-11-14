Metro - metronome for Linux (VST3 / CLAP plugin and standalone app)
====================================================================

https://github.com/greblus/metro

Contents of this package
------------------------

  Metro.vst3/   VST3 plugin (Ardour, Reaper, Bitwig, ...)
  Metro.clap    CLAP plugin (Reaper, Bitwig, ...)
  metro         standalone application (no DAW needed)
  LICENSE       GPL-3.0-or-later

Requirements: Linux x86_64 (built on Ubuntu 24.04, needs glibc 2.39 or newer),
OpenGL, X11 or XWayland. The standalone app needs JACK/PipeWire or ALSA.


Installing the plugins
----------------------

Copy them to your user plugin folders:

  mkdir -p ~/.vst3 ~/.clap
  cp -r Metro.vst3 ~/.vst3/
  cp Metro.clap ~/.clap/

(or system-wide: /usr/lib/vst3 and /usr/lib/clap, as root).
Then restart your DAW and rescan plugins if it asks.


Running the standalone app
--------------------------

  ./metro                 picks an audio backend automatically
  ./metro -b jack         JACK (also works with PipeWire's JACK support)
  ./metro -b alsa         ALSA (under PipeWire this goes through pipewire-alsa)
  ./metro --help          all options (sample rate, period size, devices, ...)

With PipeWire the outputs are usually connected to your sound card
automatically. If you hear nothing, check the connections in qpwgraph or
Helvum (or try "-b alsa"). Inputs are never connected automatically.

In the standalone app "Sync to host transport" and "Mix with track audio"
are not available - they only make sense inside a DAW.


Recording the click in a DAW
----------------------------

DAWs record a track's INPUT (before its plugins):

  Reaper: put Metro on a track, right-click the record-arm button and choose
          "Record: output (stereo)" and "Monitor input".

  Ardour: put Metro on an audio bus and connect the bus output to the input
          of the track you record on. Do not also connect a sound card input
          (capture_*) to that same track input - with the JACK backend on
          PipeWire this records silence.

"Sync to host transport" makes the click follow the project tempo and bars.


Language
--------

The UI is in English or Polish, picked from the system locale
(LC_ALL, LC_MESSAGES, LANG). Override with METRO_LANG=en or METRO_LANG=pl.


------------------------------------------------------------------------
Po polsku (skrót)
------------------------------------------------------------------------

Wtyczki: skopiuj Metro.vst3 do ~/.vst3, a Metro.clap do ~/.clap, uruchom
ponownie DAW (w razie potrzeby przeskanuj wtyczki).

Samodzielny program: ./metro (albo ./metro -b jack / -b alsa, ./metro --help).
Pod PipeWire wyjścia zwykle podłączają się same; jeśli nic nie słychać,
sprawdź połączenia w qpwgraph albo spróbuj -b alsa. Synchronizacja z hostem
i miksowanie ze ścieżką działają tylko we wtyczce.

Nagrywanie kliku: DAW nagrywa WEJŚCIE ścieżki. Reaper: "Record: output
(stereo)" + "Monitor input". Ardour: Metro na szynie, wyjście szyny na wejście
ścieżki nagrywającej, bez jednoczesnego wejścia z karty (capture_*).

Język: z locale systemu, wymuszenie przez METRO_LANG=pl lub METRO_LANG=en.

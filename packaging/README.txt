Metro - metronome for Linux (VST3 / CLAP plugin and standalone app)
====================================================================

https://github.com/greblus/metro

Contents of this package
------------------------

  Metro.vst3/   VST3 plugin (Ardour, Reaper, Bitwig, ...)
  Metro.clap    CLAP plugin (Reaper, Bitwig, ...)
  metro         standalone application (no DAW needed)
  LICENSE       ISC license

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

  ./metro                 plays through the system default output
  ./metro -b jack         JACK instead (connects to system:playback_1/2)
  ./metro --help          all options (sample rate, period size, devices, ...)

With PipeWire, Metro plays through the system default output, like other
apps, and follows it when you change it. To use a different output (USB
interface, HDMI/TV, ...) open the menu (the button with three lines) and pick
one under "Output"; the choice is remembered in ~/.config/metro/standalone.conf.

With "-b jack" the outputs go to system:playback_1/2, which under PipeWire is
the first sound card, not necessarily the one you listen on. Inputs are never
connected automatically.

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

Samodzielny program: ./metro gra na domyślne wyjście systemu (pod PipeWire)
i podąża za jego zmianą. Inne wyjście (interfejs USB, HDMI/TV...) wybierzesz
w menu (przycisk z trzema kreskami) w wierszu "Wyjście"; wybór jest
zapamiętywany. ./metro -b jack używa JACK-a (system:playback_1/2 = pierwsza
karta). Synchronizacja z hostem i miksowanie ze ścieżką działają tylko we
wtyczce.

Nagrywanie kliku: DAW nagrywa WEJŚCIE ścieżki. Reaper: "Record: output
(stereo)" + "Monitor input". Ardour: Metro na szynie, wyjście szyny na wejście
ścieżki nagrywającej, bez jednoczesnego wejścia z karty (capture_*).

Język: z locale systemu, wymuszenie przez METRO_LANG=pl lub METRO_LANG=en.

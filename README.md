# Modal
Modal is modular sound synthesiser designed for both standalone use and as
a plugin in your favorite DAW.

Modal enables flexible operations on sounds as well as low-frequency signals in
real time, where any signal can control the behavior of any block. A considerable
number of building blocks has already been implemented, including (but not
limited to):
- Various oscillators
- Noise generators
- Delays
- Filters:
  - BiQuad
  - 1st order IIR
  - One Zero
  - Pole Zero
- Instruments built on the wave guide model:
  - Glass Harmonica
  - Tibetan Prayer Bowl
  - Tuned Bar
  - Uniform Bar
  - Plucked String
- Fluidlite synth evaluator
- Arbitary arithmetic expressions
- Clipping
- Chorus
- Glide

Besides raw signals, Modal supports MIDI via `midi` files and the [JACK Audio
Connection Kit](https://jackaudio.org/) and synchronizing various blocks and
oscillators via a dedicated `Beat` signal.

# Examples
## Oscillators
The fundamental Oscillator block will cover all your needs when it comes to
oscillators. Place it by clicking RMB and start typing `oscillator` or look
under `Source` category. Click on the name to place the block. Then click `Play`
to hear the output in real-time.

Change the frequency by changing the number in the frequency input and
preview the produced signal by clicking on `Scope`.

![Osc1](https://raw.githubusercontent.com/kamirr/modal/main/screenshots/osc-1.png)
![Osc2](https://raw.githubusercontent.com/kamirr/modal/main/screenshots/osc-2.png)

Change the shape of the waveform by turning the knob. In-between values
are also supported in which case Modal will automatically interpolate between
the waveforms in a way that makes sense™.

![Osc3](https://raw.githubusercontent.com/kamirr/modal/main/screenshots/osc-3.png)
![Osc4](https://raw.githubusercontent.com/kamirr/modal/main/screenshots/osc-4.png)

Change the value range of the oscillator by checking `Manual range`,
which enables two extra inputs. This is useful for modulating the behavior of
other blocks, like changing the frequency of another oscillator in time (vibrato)
or its shape.

![Osc6](https://raw.githubusercontent.com/kamirr/modal/main/screenshots/osc-5.png)
![Osc7](https://raw.githubusercontent.com/kamirr/modal/main/screenshots/osc-6.png)
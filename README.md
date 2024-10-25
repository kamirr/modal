# Modal
Modal is a standalone, modular sound synthesiser.

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

Besides raw signals, Modal supports MIDI via `midi` files and the [JACK Audio
Connection Kit](https://jackaudio.org/) and synchronizing various blocks and
oscillators via a dedicated `Beat` signal.

# Examples
Soon™
# 💬 resyn

*TODO: Write description.*

## 🔧 Installation

1. Clone the repository and enter it.

```sh
git clone https://github.com/7ap/resyn.git && cd resyn
```

2. Download [vosk-api](https://github.com/alphacep/vosk-api/releases/tag/v0.3.45) and extract the contents to `res/vosk`.

```sh
wget https://github.com/alphacep/vosk-api/releases/download/v0.3.45/vosk-linux-x86_64-0.3.45.zip &&
unzip -q vosk-linux-x86_64-0.3.45.zip &&
mv vosk-linux-x86_64-0.3.45 res/vosk &&
rm vosk-linux-x86_64-0.3.45.zip
```

3. Download a [model](https://alphacephei.com/vosk/models) to the `res` directory. [vosk-model-small-en-us-0.15](https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip) seems to work the best.

```sh
wget https://alphacephei.com/vosk/models/vosk-model-small-en-us-0.15.zip &&
unzip -q vosk-model-small-en-us-0.15.zip &&
mv vosk-model-small-en-us-0.15 res &&
rm vosk-model-small-en-us-0.15.zip
```

4. Install `tts-server` from the [🐸TTS](https://github.com/coqui-ai/tts) Python package.

```sh
pip3 install TTS # Requires Python <=3.10
```

## ⚙️ Usage

1. Run `tts-server` with a model of your choice. [VITS](https://arxiv.org/pdf/2106.06103.pdf) seems to work the best overall. **This must be running in the background to synthesize speech.**

```sh
tts-server --model_name tts_models/en/vctk/vits
```

2. Run `resyn`.

```sh
cargo run -- --help
```

### Virtual Microphone

1. Install [qpwgraph](https://github.com/rncbc/qpwgraph).

```sh
flatpak install flathub org.rncbc.qpwgraph
```

2. Run `./res/pipewire`. This will create the `virtual-sink` and `virtual-mic` devices.

```sh
./res/pipewire # Must be run once after every restart.
```

3. Run `qpwgraph` using the `res/resyn.qpwgraph` patchbay.

```sh
nohup qpwgraph --activated ./res/resyn.qpwgraph > /dev/null 2>&1 & # Adding `--exclusive` after `--activated` will mute the synthesized voice in your default output device.
```

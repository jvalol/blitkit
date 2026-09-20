/// Plays sounds on the default output device. When no output device can be
/// opened, every sound is silently dropped instead.
pub struct SoundSystem {
    output: Option<Output>,
}

struct Output {
    player: rodio::Player,
    spatial_player: rodio::SpatialPlayer,
    // Playback stops when this is dropped, so it lives as long as the players.
    _sink: rodio::MixerDeviceSink,
}

impl Default for SoundSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl SoundSystem {
    pub fn new() -> Self {
        let output = match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(mut sink) => {
                sink.log_on_drop(false);

                let player = rodio::Player::connect_new(sink.mixer());
                player.set_volume(0.5);

                let spatial_player = rodio::SpatialPlayer::connect_new(
                    sink.mixer(),
                    [0.0, 0.0, 0.0],
                    [-1.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                );

                Some(Output {
                    player,
                    spatial_player,
                    _sink: sink,
                })
            }
            Err(e) => {
                log::warn!(
                    "Could not open an audio output device, sound is disabled: {}",
                    e
                );
                None
            }
        };

        Self { output }
    }

    #[inline]
    pub fn queue<S>(&self, sound: S)
    where
        S: rodio::Source + Send + 'static,
    {
        if let Some(output) = &self.output {
            output.player.append(sound);
        }
    }

    #[allow(dead_code)]
    #[inline]
    pub fn queue_spatial<S>(&self, sound: S, position: [f32; 3])
    where
        S: rodio::Source + Send + 'static,
    {
        if let Some(output) = &self.output {
            output.spatial_player.set_emitter_position(position);
            output.spatial_player.append(sound);
        }
    }
}

/// Plays sounds on the default output device. When there is no output device,
/// every sound is silently dropped instead.
pub struct SoundSystem {
    output: Option<Output>,
}

struct Output {
    #[allow(dead_code)]
    device: rodio::Device,
    sink: rodio::Sink,
    spatial_sink: rodio::SpatialSink,
}

impl SoundSystem {
    pub fn new() -> Self {
        let output = match rodio::default_output_device() {
            Some(device) => {
                let sink = rodio::Sink::new(&device);
                sink.set_volume(0.5);

                let spatial_sink = rodio::SpatialSink::new(
                    &device,
                    [0.0, 0.0, 0.0],
                    [-1.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                );

                Some(Output {
                    device,
                    sink,
                    spatial_sink,
                })
            }
            None => {
                log::warn!("No audio output device found, sound is disabled");
                None
            }
        };

        Self { output }
    }

    #[inline]
    pub fn queue<S>(&self, sound: S)
    where
        S: rodio::Source + Send + 'static,
        S::Item: rodio::Sample,
        S::Item: Send,
    {
        if let Some(output) = &self.output {
            output.sink.append(sound);
        }
    }

    #[allow(dead_code)]
    #[inline]
    pub fn queue_spatial<S>(&self, sound: S, position: [f32; 3])
    where
        S: rodio::Source + Send + 'static,
        S::Item: rodio::Sample + Send + std::fmt::Debug,
    {
        if let Some(output) = &self.output {
            output.spatial_sink.set_emitter_position(position);
            output.spatial_sink.append(sound);
        }
    }
}

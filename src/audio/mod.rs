use rodio::microphone::MicrophoneBuilder;
use rodio::speakers::SpeakersBuilder;
use tokio::sync::oneshot;

pub async fn run_mic_to_speaker(rx: oneshot::Receiver<()>, mic: rodio::microphone::Input, speaker: rodio::speakers::Output, channels: u16) {
    let microphone = MicrophoneBuilder::new()
        .device(mic).unwrap()
        .default_config().unwrap()
        .try_channels(channels.try_into().unwrap()).unwrap()
        .open_stream().unwrap();

    let mut speaker = SpeakersBuilder::new()
        .device(speaker).unwrap()
        .default_config().unwrap()
        .open_mixer().unwrap();
    speaker.log_on_drop(false);
    let mixer = speaker.mixer();

    mixer.add(microphone);

    let _ = rx.await;
}

pub fn stop_mic_to_speaker(tx: oneshot::Sender<()>) {
    let _ = tx.send(());
}

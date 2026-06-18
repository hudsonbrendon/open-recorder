import { formatElapsed } from "../lib/format";

export function RecordControls(props: {
  isRecording: boolean; elapsed: number; disabled: boolean;
  onStart: () => void; onStop: () => void;
}) {
  return (
    <div className="controls">
      <button
        className={props.isRecording ? "btn stop" : "btn record"}
        disabled={props.disabled && !props.isRecording}
        onClick={props.isRecording ? props.onStop : props.onStart}>
        {props.isRecording ? "Parar" : "Gravar"}
      </button>
      {props.isRecording && <span className="timer">{formatElapsed(props.elapsed)}</span>}
    </div>
  );
}

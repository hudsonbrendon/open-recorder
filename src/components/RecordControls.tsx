import { formatElapsed } from "../lib/format";

export function RecordControls(props: {
  isRecording: boolean;
  elapsed: number;
  disabled: boolean;
  onStart: () => void;
  onStop: () => void;
}) {
  return (
    <div className="record-bar">
      <button
        className={"record-btn" + (props.isRecording ? " recording" : "")}
        disabled={props.disabled && !props.isRecording}
        onClick={props.isRecording ? props.onStop : props.onStart}
      >
        <span className="record-dot" />
        <span>{props.isRecording ? "Parar" : "Gravar"}</span>
      </button>
      {props.isRecording && (
        <span className="record-timer">{formatElapsed(props.elapsed)}</span>
      )}
    </div>
  );
}

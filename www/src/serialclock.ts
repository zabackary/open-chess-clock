import Clock from "./clock";
import { JsSerialHandler } from "./serial";

export default class SerialClock extends Clock {
  readonly = true;

  onNewMessage: NonNullable<JsSerialHandler["onNewMessage"]> = (
    message,
    args
  ) => {
    switch (message) {
      case "startP1": {
        this.p2Time = args[0];
        this.startPlayer("p1");
        break;
      }
      case "startP2": {
        this.p1Time = args[0];
        this.startPlayer("p2");
        break;
      }
      case "sync": {
        if (this.currentPlayer === null) {
          // game stopped. update initial times too
          this.p1TimeInitial = args[0];
          this.p2TimeInitial = args[1];
        }
        this.updateTimes(args[0], args[1]);
        break;
      }
      case "p1Finish": {
        this.loser = "p1";
        this.p1Time = 0;
        break;
      }
      case "p2Finish": {
        this.loser = "p2";
        this.p2Time = 0;
        break;
      }
      case "pause": {
        this.pause();
        break;
      }
    }
  };

  get connected() {
    return this.serialHandler.connected;
  }

  constructor(private serialHandler: JsSerialHandler) {
    super(0, 0);
    console.log(this);
    serialHandler.onNewMessage = this.onNewMessage.bind(this);
  }
}

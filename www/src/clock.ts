// @ts-check

type Player = "p1" | "p2";

export default class Clock {
  private p1TimeStart: number = 0;
  private p1TimeStartDate: Date | null = null;
  private p2TimeStart: number = 0;
  private p2TimeStartDate: Date | null = null;

  p1TimeInitial = 0;
  p2TimeInitial = 0;

  loser: Player | null = null;

  readonly = false;

  get p1Time() {
    if (this.p1TimeStartDate === null) {
      return this.p1TimeStart;
    }
    let time =
      this.p1TimeStart -
      (new Date().getTime() - this.p1TimeStartDate.getTime());
    if (time <= 0) {
      time = 0;
    }
    return time;
  }

  set p1Time(newTime) {
    this.p1TimeStart = newTime;
    if (this.p1TimeStartDate !== null) this.p1TimeStartDate = new Date();
  }

  get p2Time() {
    if (this.p2TimeStartDate === null) {
      return this.p2TimeStart;
    }
    let time =
      this.p2TimeStart -
      (new Date().getTime() - this.p2TimeStartDate.getTime());
    if (time <= 0) {
      time = 0;
    }
    return time;
  }

  set p2Time(newTime) {
    this.p2TimeStart = newTime;
    if (this.p2TimeStartDate !== null) this.p2TimeStartDate = new Date();
  }

  updateTimes(newP1Time: number, newP2Time: number) {
    this.p1Time = newP1Time;
    this.p2Time = newP2Time;
  }

  pause() {
    this.p1TimeStart = this.p1Time;
    this.p1TimeStartDate = null;
    this.p2TimeStart = this.p2Time;
    this.p2TimeStartDate = null;
  }

  checkForLoser(): Player | null {
    if (this.p1Time === 0) {
      this.loser = "p1";
    } else if (this.p2Time === 0) {
      this.loser = "p2";
    }
    return this.loser;
  }

  startPlayer(player: Player, time: number | null = null) {
    if (player === "p1") {
      if (this.p1TimeStartDate === null) {
        this.p1TimeStartDate = new Date();
      }
      if (time !== null) {
        this.p1Time = time;
      }
      this.p2TimeStart = this.p2Time;
      this.p2TimeStartDate = null;
    } else {
      if (this.p2TimeStartDate === null) {
        this.p2TimeStartDate = new Date();
      }
      if (time !== null) {
        this.p2Time = time;
      }
      this.p1TimeStart = this.p1Time;
      this.p1TimeStartDate = null;
    }
  }

  get currentPlayer(): Player | null {
    if (this.p1TimeStartDate === null && this.p2TimeStartDate !== null) {
      return "p2";
    } else if (this.p2TimeStartDate === null && this.p1TimeStartDate !== null) {
      return "p1";
    } else {
      return null;
    }
  }

  /**
   * Converts ms to a minute-second pair
   */
  static msToMS(ms: number): [number, number] {
    return [
      Math.floor(ms / (1000 * 60)),
      Math.floor((ms % (1000 * 60)) / 1000),
    ];
  }

  constructor(p1Time: number, p2Time: number) {
    this.p1TimeInitial = p1Time;
    this.p2TimeInitial = p2Time;
    this.updateTimes(p1Time, p2Time);
  }
}

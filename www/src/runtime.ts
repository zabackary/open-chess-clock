// @ts-check

import Clock from "./clock";
import { expectEl } from "./main";
import SerialClock from "./serialclock";
import "./style.css";

const READONLY_CONNECTING_STATUS = "connecting...";
const READONLY_STATUS = "connected over USB";
const READONLY_START_HINT =
  "Use the physical clock to set the time. The time will mirror here";
const START_HINT = "Press a side to start the clock";
const RESUME_HINT = "Press a side to resume";
const READONLY_RESUME_HINT =
  "Press a side to resume. Pressing START will start a new game";
const SWITCH_HINT = "Press your side to switch the clock";

const winner = expectEl(document.querySelector("#winner"));
const counterContainer = expectEl(document.querySelector(".counter-container"));
const p1Counter = expectEl(document.querySelector(".p1.counter"));
const p1TimeElements = [
  document.querySelector(".p1.counter .minutes"),
  document.querySelector(".p1.counter .seconds"),
].map((x) => expectEl(x));
const p2Counter = expectEl(document.querySelector(".p2.counter"));
const p2TimeElements = [
  document.querySelector(".p2.counter .minutes"),
  document.querySelector(".p2.counter .seconds"),
].map((x) => expectEl(x));

const pauseButton = expectEl(document.querySelector("#pause-button"));
const restartButton = expectEl(document.querySelector("#restart-button"));
const status = expectEl(document.querySelector(".status"));
const hint = expectEl(document.querySelector("#hint"));
const controls = expectEl(document.querySelector("#controls"));

export function runtime(clock: Clock) {
  status.textContent = "";
  if (clock.readonly) {
    p1Counter.setAttribute("disabled", "");
    p2Counter.setAttribute("disabled", "");
    controls.classList.add("hidden");
  } else {
    p1Counter.removeAttribute("disabled");
    p2Counter.removeAttribute("disabled");
    pauseButton.setAttribute("disabled", "");
    controls.classList.remove("hidden");
  }
  let hasStarted = false;
  const updateInterval = setInterval(() => {
    if (clock instanceof SerialClock) {
      status.textContent = clock.connected
        ? READONLY_STATUS
        : READONLY_CONNECTING_STATUS;
      if (clock.connected) {
        counterContainer.classList.remove("show-connecting");
      }
    }
    Clock.msToMS(clock.p1Time).map((component, i) => {
      p1TimeElements[i].textContent = component
        .toString()
        .padStart(i === 0 ? 1 : 2, "0");
    });
    p1Counter.style.setProperty(
      "--progress",
      (clock.p1Time / clock.p1TimeInitial).toString()
    );
    Clock.msToMS(clock.p2Time).map((component, i) => {
      p2TimeElements[i].textContent = component
        .toString()
        .padStart(i === 0 ? 1 : 2, "0");
    });
    p2Counter.style.setProperty(
      "--progress",
      (clock.p2Time / clock.p2TimeInitial).toString()
    );
    const currentPlayer = clock.currentPlayer;
    if (currentPlayer === "p1") {
      p1Counter.classList.add("active");
      p2Counter.classList.remove("active");
      if (!clock.readonly) {
        p1Counter.removeAttribute("disabled");
        p2Counter.setAttribute("disabled", "");
        pauseButton.removeAttribute("disabled");
      }
      hint.textContent = SWITCH_HINT;
    } else if (currentPlayer === "p2") {
      p1Counter.classList.remove("active");
      p2Counter.classList.add("active");
      if (!clock.readonly) {
        p1Counter.setAttribute("disabled", "");
        p2Counter.removeAttribute("disabled");
        pauseButton.removeAttribute("disabled");
      }
      hint.textContent = SWITCH_HINT;
    } else {
      p1Counter.classList.remove("active");
      p2Counter.classList.remove("active");
      if (clock.readonly) {
        hint.textContent = hasStarted
          ? READONLY_RESUME_HINT
          : READONLY_START_HINT;
      } else {
        hint.textContent = hasStarted ? RESUME_HINT : START_HINT;
        p1Counter.removeAttribute("disabled");
        p2Counter.removeAttribute("disabled");
        pauseButton.setAttribute("disabled", "");
      }
    }
    if (!clock.readonly) clock.checkForLoser();
    if (clock.loser !== null) {
      clearInterval(updateInterval);
      winner.textContent = clock.loser === "p1" ? "Player 2" : "Player 1";
      counterContainer.classList.add("show-winner");
    }
  });
  pauseButton.addEventListener("click", () => {
    clock.pause();
  });
  p1Counter.addEventListener("click", () => {
    hasStarted = true;
    clock.startPlayer(clock.currentPlayer === null ? "p1" : "p2");
  });
  p2Counter.addEventListener("click", () => {
    hasStarted = true;
    clock.startPlayer(clock.currentPlayer === null ? "p2" : "p1");
  });
  restartButton.addEventListener("click", () => {
    hasStarted = false;
    clock.updateTimes(clock.p1TimeInitial, clock.p2TimeInitial);
    clock.pause();
  });
}

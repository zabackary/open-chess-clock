// @ts-check

import Clock from "./clock";
import { runtime } from "./runtime";
import { JsSerialHandler } from "./serial";
import SerialClock from "./serialclock";
import "./style.css";

const SERIAL_ARDUINO_USB_VENDOR_ID = 0x2341;
const SERIAL_BAUD_RATE = 57600;

export function expectEl(maybeHTMLElement: Element | null) {
  if (!(maybeHTMLElement instanceof HTMLElement)) {
    throw new Error(`${maybeHTMLElement} is not an element`);
  }
  return maybeHTMLElement;
}

const counterContainer = expectEl(document.querySelector(".counter-container"));
const connectButton = expectEl(document.querySelector("#connect-button"));
const customButton = expectEl(document.querySelector(".text-button.custom"));
const customDialog = expectEl(document.querySelector(".custom-dialog"));
const customCancel = expectEl(document.querySelector("#custom-cancel-button"));
const customStart = expectEl(document.querySelector("#custom-start-button"));
const presetButtons = [...document.querySelectorAll(".preset")].map(expectEl);
const p1TimeSetElements = [
  ...document.querySelectorAll(".time-select.p1 input"),
].map(expectEl) as HTMLInputElement[];
const p2TimeSetElements = [
  ...document.querySelectorAll(".time-select.p2 input"),
].map(expectEl) as HTMLInputElement[];

async function runSerial() {
  counterContainer.classList.remove("show-picker");
  counterContainer.classList.add("show-connecting");
  const port = await navigator.serial.requestPort({
    filters: [
      {
        usbVendorId: SERIAL_ARDUINO_USB_VENDOR_ID,
      },
    ],
  });
  await port.open({
    baudRate: SERIAL_BAUD_RATE,
  });
  const serialHandler = new JsSerialHandler(port);
  // We don't do a handshake here because we expect the hardware to
  runtime(new SerialClock(serialHandler));
}

async function run(time: number) {
  counterContainer.classList.remove("show-picker");
  runtime(new Clock(time, time));
}

connectButton.addEventListener("click", () => {
  runSerial();
});

customButton.addEventListener("click", () => {
  customDialog.classList.add("open");
});

customStart.addEventListener("click", () => {
  const coerceToZero = (x: string) => (x === "" ? "0" : x);
  const p1Ms =
    parseInt(coerceToZero(p1TimeSetElements[0].value), 10) * 60 * 1000 +
    parseInt(coerceToZero(p1TimeSetElements[1].value), 10) * 1000;
  const p2Ms =
    parseInt(coerceToZero(p2TimeSetElements[0].value), 10) * 60 * 1000 +
    parseInt(coerceToZero(p2TimeSetElements[1].value), 10) * 1000;
  if (p1Ms > 0 && p2Ms > 0) {
    customDialog.classList.remove("open");
    counterContainer.classList.remove("show-picker");
    runtime(new Clock(p1Ms, p2Ms));
  }
});

customCancel.addEventListener("click", () => {
  customDialog.classList.remove("open");
});

presetButtons.forEach((btn) => {
  btn.addEventListener("click", () => {
    run(parseInt(btn.dataset.time ?? "0", 10));
  });
});

function checkTimeElement(e: Event) {
  const el = e.currentTarget as HTMLInputElement;
  const value = parseInt(el.value, 10);
  if (value > parseInt(el.max, 10)) {
    e.preventDefault();
    el.value = el.max;
  } else if (value < parseInt(el.min, 10)) {
    e.preventDefault();
    el.value = el.min;
  }
}
p1TimeSetElements[0].addEventListener("input", checkTimeElement);
p1TimeSetElements[1].addEventListener("input", checkTimeElement);
p2TimeSetElements[0].addEventListener("input", checkTimeElement);
p2TimeSetElements[1].addEventListener("input", checkTimeElement);

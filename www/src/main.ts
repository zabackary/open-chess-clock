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
].map(expectEl);
const p2TimeSetElements = [
  ...document.querySelectorAll(".time-select.p2 input"),
].map(expectEl);

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
  customDialog.classList.remove("open");
  counterContainer.classList.remove("show-picker");
  const p1Ms =
    parseInt((p1TimeSetElements[0] as HTMLInputElement).value, 10) * 60 * 1000 +
    parseInt((p1TimeSetElements[1] as HTMLInputElement).value, 10) * 1000;
  const p2Ms =
    parseInt((p2TimeSetElements[0] as HTMLInputElement).value, 10) * 60 * 1000 +
    parseInt((p2TimeSetElements[1] as HTMLInputElement).value, 10) * 1000;
  runtime(new Clock(p1Ms, p2Ms));
});

customCancel.addEventListener("click", () => {
  customDialog.classList.remove("open");
});

presetButtons.forEach((btn) => {
  btn.addEventListener("click", () => {
    run(parseInt(btn.dataset.time ?? "0", 10));
  });
});

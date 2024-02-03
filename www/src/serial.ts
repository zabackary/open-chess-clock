/**
 * The types of sendable messages.
 * Keep up to date with /src/serial.rs
 *
 * Note: all messages are big-endian
 */
const messages = Object.freeze({
  handshake: {
    hex: 0xc0,
    arguments: 1,
  },
  handshakeResponse: {
    hex: 0xc1,
    arguments: 1,
  },
  startP1: {
    hex: 0xc2,
    arguments: 1,
  },
  startP2: {
    hex: 0xc3,
    arguments: 1,
  },
  sync: {
    hex: 0xc4,
    arguments: 2,
  },
  pause: {
    hex: 0xc5,
    arguments: 1,
  },
  p1Finish: {
    hex: 0xc6,
    arguments: 0,
  },
  p2Finish: {
    hex: 0xc7,
    arguments: 0,
  },
} satisfies Record<string, { hex: number; arguments: number }>);

/**
 * Handles serial communication between the firmware and website.
 * Keep up to date with /src/serial.rs
 */
export class SerialHandler {
  connected = false;

  constructor(private port: SerialPort) {}

  async write(message: keyof typeof messages, args: number[] = []) {
    const writer = this.port.writable?.getWriter();
    if (!writer) throw new Error("failed to lock serial writer");
    await writer.ready;
    if (args.length !== messages[message].arguments) {
      throw new Error("provided arguments does not match spec");
    }
    const buffer = new ArrayBuffer(
      1 + // the msg header
        args.length * 4 // one u32 (4 bytes) per argument
    );
    const dv = new DataView(buffer, 0, buffer.byteLength);
    dv.setUint8(0, messages[message].hex);
    args.forEach((arg, i) => {
      dv.setUint32(
        1 + i * 4,
        arg,
        false // big-endian
      );
    });
    await writer.write(new Uint8Array(buffer));
    writer.releaseLock();
    console.info(`wrote ${message}:`, args);
  }

  private async rawRead(): Promise<[keyof typeof messages, number[]]> {
    const read = async (): Promise<[keyof typeof messages, number[]]> => {
      const reader = this.port.readable?.getReader({
        mode: "byob",
      });
      if (!reader) throw new Error("failed to lock serial writer");
      const { value: headerValue } = await reader.read(
        new Uint8Array(new ArrayBuffer(1), 0, 1)
      );
      if (!headerValue) throw new Error("stream ended early");
      const headerDv = new DataView(headerValue.buffer);
      const msgId = headerDv.getUint8(0);
      const message = Object.entries(messages).find(
        ([_, { hex }]) => hex === msgId
      )?.[0] as keyof typeof messages | undefined;
      if (message === undefined) {
        console.warn("unknown serial message header:", msgId);
        // wait for a little, flush reader for some amount of bytes then try again
        await new Promise((r) => setTimeout(r, 100));
        await reader.read(new Uint8Array(256));
        reader.releaseLock();
        return await read();
      }
      if (messages[message].arguments > 0) {
        // Wait for a little to let the clock finish sending the bytes
        await new Promise((r) => setTimeout(r, 100));
        const argsLength = messages[message].arguments * 4;
        const argumentsBuffer = new ArrayBuffer(argsLength);
        const { value: argumentsValue } = await reader.read(
          new Uint8Array(argumentsBuffer, 0, argsLength)
        );
        if (!argumentsValue) throw new Error("stream ended early");
        const argumentsDv = new DataView(argumentsValue.buffer);
        const args: number[] = [];
        for (let offset = 0; offset < argsLength; offset += 4) {
          args.push(
            argumentsDv.getUint32(
              offset,
              false // big-endian
            )
          );
        }
        reader.releaseLock();
        console.info(`read ${message}:`, args);
        return [message, args];
      } else {
        reader.releaseLock();
        console.info(`read ${message}`);
        return [message, []];
      }
    };
    return await read();
  }

  async read(): Promise<[keyof typeof messages, number[]]> {
    const read = async (): Promise<[keyof typeof messages, number[]]> => {
      const [msg, args] = await this.rawRead();
      if (msg === "handshake") {
        this.connected = true;
        let selectedMode = 0x0000;
        switch (args[0]) {
          case 0x0000: {
            // We decide
            selectedMode = 0x0003; // make other party master
            break;
          }
          case 0x0001: {
            // Sync
            selectedMode = 0x0000; // unsupported
            break;
          }
          case 0x0002: {
            // We're slave
            selectedMode = 0x0002; // confirmed
            break;
          }
          case 0x0003: {
            // We're master
            selectedMode = 0x0000; // Unsupported
            break;
          }
          default: {
            selectedMode = 0x0000; // Unsupported
            break;
          }
        }
        this.write("handshakeResponse", [selectedMode]);
        // retry
        return await read();
      } else if (msg === "handshakeResponse") {
        this.connected = true;
        if (args[0] === 0x0000) {
          // they said our mode is unsupported
          console.error("can't negotiate mode");
        }
        // retry
        return await read();
      } else {
        return [msg, args];
      }
    };
    return await read();
  }

  async close() {
    this.port.close();
  }

  async open(baudRate: number) {
    this.port.open({
      baudRate,
    });
  }
}

/**
 * SerialHandler made more JavaScripty (since the original was written in Rust)
 */
export class JsSerialHandler extends SerialHandler {
  onNewMessage:
    | ((message: keyof typeof messages, args: number[]) => void)
    | undefined;

  constructor(port: SerialPort) {
    super(port);

    const readLoop = async () => {
      while (true) {
        const [message, args] = await this.read();
        this.onNewMessage?.call(this, message, args);
      }
    };
    readLoop();
  }
}

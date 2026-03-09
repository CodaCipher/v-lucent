export interface EmotionParams {
  "Head:: Yaw-Pitch"?: [number, number];
  "Body:: Yaw-Pitch"?: [number, number];
  "Mouth:: Shape"?: [number, number];
  "Mouth:: Width"?: [number, number];
  "Eye:: Left:: Blink"?: [number, number];
  "Eye:: Right:: Blink"?: [number, number];
  "Body:: Roll"?: [number, number];
  "Arm:: Left:: Move"?: [number, number];
  "Arm:: Right:: Move"?: [number, number];
  [key: string]: [number, number] | undefined;
}

export const EMOTION_PRESETS: Record<string, EmotionParams> = {
  neutral: {},

  happy: {
    "Mouth:: Shape": [0.6, 0],
    "Mouth:: Width": [0.3, 0],
    "Eye:: Left:: Blink": [0.15, 0],
    "Eye:: Right:: Blink": [0.15, 0],
    "Head:: Yaw-Pitch": [0, -0.1],
    "Body:: Yaw-Pitch": [0, -0.05],
  },

  sad: {
    "Mouth:: Shape": [-0.4, 0],
    "Mouth:: Width": [-0.1, 0],
    "Head:: Yaw-Pitch": [0, 0.15],
    "Body:: Yaw-Pitch": [0, 0.08],
    "Body:: Roll": [-0.1, 0],
  },

  angry: {
    "Mouth:: Shape": [-0.3, 0],
    "Mouth:: Width": [0.2, 0],
    "Head:: Yaw-Pitch": [0, -0.12],
    "Body:: Yaw-Pitch": [0, -0.06],
  },

  surprised: {
    "Mouth:: Shape": [0.5, 0],
    "Mouth:: Width": [0.4, 0],
    "Head:: Yaw-Pitch": [0, -0.15],
    "Body:: Yaw-Pitch": [0, -0.08],
  },

  thinking: {
    "Head:: Yaw-Pitch": [0.25, 0.1],
    "Body:: Yaw-Pitch": [0.1, 0.05],
    "Eye:: Left:: Blink": [0.1, 0],
    "Eye:: Right:: Blink": [0.3, 0],
  },
};

export class EmotionController {
  private current: Record<string, [number, number]> = {};
  private target: Record<string, [number, number]> = {};
  private lerpSpeed: number;

  constructor(lerpSpeed: number = 3.0) {
    this.lerpSpeed = lerpSpeed;
  }

  setEmotion(emotion: string, intensity: number = 1.0) {
    const preset = EMOTION_PRESETS[emotion] ?? EMOTION_PRESETS["neutral"];
    this.target = {};

    for (const [key, val] of Object.entries(preset)) {
      if (val) {
        this.target[key] = [val[0] * intensity, val[1] * intensity];
      }
    }
  }

  update(dt: number): Record<string, [number, number]> {
    const allKeys = new Set([
      ...Object.keys(this.current),
      ...Object.keys(this.target),
    ]);

    const factor = 1 - Math.exp(-this.lerpSpeed * dt);

    for (const key of allKeys) {
      const cur = this.current[key] ?? [0, 0];
      const tgt = this.target[key] ?? [0, 0];

      const newX = cur[0] + (tgt[0] - cur[0]) * factor;
      const newY = cur[1] + (tgt[1] - cur[1]) * factor;

      if (Math.abs(newX) < 0.001 && Math.abs(newY) < 0.001 && !this.target[key]) {
        delete this.current[key];
      } else {
        this.current[key] = [newX, newY];
      }
    }

    return { ...this.current };
  }
}

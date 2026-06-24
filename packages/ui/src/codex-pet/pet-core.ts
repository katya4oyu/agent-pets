// 時間ベースでスプライトのフレームを進める純TSコア。
// codex-pet-web の MoonBit 実装（moon/pet_core.mbt）を移植したもの。
// MoonBit は JS ターゲットの軽量な整数演算だったため、パフォーマンス差なく純TSへ置換できる。

export type CoreAnimation = {
  fps: number;
  frameCount: number;
};

export type PetCoreInput = {
  animations: Record<string, CoreAnimation>;
  dragDirectionX: number;
  dragging: boolean;
  fpsOverride?: number;
  now: number;
  requestedAnimation?: string;
};

export type AnimationPlayback = {
  animation: string;
  frameIndex: number;
};

export type PetCore = {
  tick(input: PetCoreInput): AnimationPlayback;
};

// MoonBit enum PetAnimation の並びと一致させる（index がそのまま state.animation）。
const animationNames = [
  "idle",
  "running-right",
  "running-left",
  "waving",
  "jumping",
  "failed",
  "waiting",
  "running",
  "review",
] as const;

const animationAliases: Record<string, string> = {
  jump: "jumping",
  play: "waving",
  sleep: "waiting",
  walk: "running",
};

// moon/pet_core.mbt: animation_from_name
function animationIndexFromName(name: string): number {
  switch (name) {
    case "running-right":
      return 1;
    case "running-left":
      return 2;
    case "waving":
    case "play":
      return 3;
    case "jumping":
    case "jump":
      return 4;
    case "failed":
      return 5;
    case "waiting":
    case "sleep":
      return 6;
    case "running":
    case "walk":
      return 7;
    case "review":
      return 8;
    default:
      return 0; // idle
  }
}

type PetState = {
  animation: number;
  frameIndex: number;
  lastFrameMs: number;
};

// moon/pet_core.mbt: choose_animation
function chooseAnimation(requested: number, dragging: boolean, dragDirectionX: number): number {
  if (dragging) {
    return dragDirectionX < 0 ? 2 /* running-left */ : 1 /* running-right */;
  }
  return requested;
}

// moon/pet_core.mbt: should_advance
function shouldAdvance(nowMs: number, lastFrameMs: number, fps: number): boolean {
  return nowMs - lastFrameMs >= 1000 / Math.max(fps, 1);
}

function chooseRequestedAnimation(input: PetCoreInput): string {
  const requested = input.requestedAnimation;
  const canonical = requested ? animationAliases[requested] ?? requested : undefined;
  if (canonical && input.animations[canonical]) {
    return canonical;
  }
  return "idle";
}

export function createPetCore(): PetCore {
  let state: PetState = { animation: 0, frameIndex: 0, lastFrameMs: 0 };

  return {
    tick(input) {
      const requestedName = chooseRequestedAnimation(input);
      const requested = animationIndexFromName(requestedName);
      const activeAnimation = animationNames[requested] ?? "idle";
      const draggingAnimation = input.dragDirectionX < 0 ? "running-left" : "running-right";
      const config =
        input.animations[input.dragging ? draggingAnimation : activeAnimation] ?? input.animations.idle;

      if (!config) {
        return { animation: "idle", frameIndex: 0 };
      }

      const fps = input.fpsOverride ?? config.fps;
      const now = Math.trunc(input.now);
      const animation = chooseAnimation(requested, input.dragging, Math.sign(input.dragDirectionX));

      // moon/pet_core.mbt: tick
      if (state.animation !== animation) {
        state = { animation, frameIndex: 0, lastFrameMs: now };
      } else if (shouldAdvance(now, state.lastFrameMs, fps)) {
        state = {
          animation,
          frameIndex: (state.frameIndex + 1) % Math.max(config.frameCount, 1),
          lastFrameMs: now,
        };
      }

      return {
        animation: animationNames[state.animation] ?? "idle",
        frameIndex: state.frameIndex,
      };
    },
  };
}

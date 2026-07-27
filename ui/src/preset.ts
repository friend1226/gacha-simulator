import type { ModelIr } from "./types";

export const blueArchive: ModelIr = {
  irVersion: 1,
  name: "블루 아카이브 픽업 모집",
  entities: [
    {
      id: "star3",
      name: "3성",
      prob: { lit: "0.03" },
      children: [
        { id: "pickup", name: "픽업", prob: { lit: "0.007" } },
      ],
    },
  ],
  nestingPolicy: "clampChildren",
  stateVars: [],
  probRules: [],
  transitions: [],
  triggers: [
    {
      at: { trialCount: 200 },
      grant: {
        leaf: "pickup",
        amount: 1,
        consumesTrial: false,
        appliesTransitions: true,
      },
    },
  ],
  run: {
    maxTrials: 200,
    trackJoint: ["pickup", "star3__self"],
    numeric: "scaled",
  },
};


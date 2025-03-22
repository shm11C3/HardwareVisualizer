import { atom } from "jotai";

export const modalAtoms = {
  showSettingsModal: atom<boolean>(false),
};

export const settingAtoms = {
  isRequiredRestart: atom<boolean>(false),
};

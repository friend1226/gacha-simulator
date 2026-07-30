/** @vitest-environment jsdom */

import { flushSync } from "react-dom";
import ReactDOM from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  AppErrorBoundary,
  clearStoredModelAndReload,
} from "./AppErrorBoundary";
import { MODEL_STORAGE } from "./storage";

const roots: ReactDOM.Root[] = [];

afterEach(() => {
  for (const root of roots.splice(0)) root.unmount();
  document.body.replaceChildren();
  vi.restoreAllMocks();
});

describe("AppErrorBoundary", () => {
  it("shows recovery actions and the original error message", () => {
    const reloadPage = vi.fn();
    const clearStoredModel = vi.fn();
    const consoleError = vi.spyOn(console, "error").mockImplementation(() => undefined);
    const preventExpectedWindowError = (event: ErrorEvent) => event.preventDefault();
    window.addEventListener("error", preventExpectedWindowError);
    const container = document.createElement("div");
    document.body.append(container);
    const root = ReactDOM.createRoot(container);
    roots.push(root);

    function BrokenApp(): never {
      throw new Error("intentional boundary check");
    }

    flushSync(() => {
      root.render(
        <AppErrorBoundary
          reloadPage={reloadPage}
          clearStoredModel={clearStoredModel}
        >
          <BrokenApp />
        </AppErrorBoundary>,
      );
    });

    expect(container.textContent).toContain("앱을 표시하는 중 문제가 발생했습니다.");
    expect(container.textContent).toContain("intentional boundary check");
    const buttons = [...container.querySelectorAll("button")];
    expect(buttons.map((button) => button.textContent)).toEqual([
      "새로고침",
      "저장된 모델 지우고 다시 시작",
    ]);
    buttons[0].click();
    buttons[1].click();
    expect(reloadPage).toHaveBeenCalledOnce();
    expect(clearStoredModel).toHaveBeenCalledOnce();
    expect(consoleError).toHaveBeenCalled();
    window.removeEventListener("error", preventExpectedWindowError);
  });

  it("reloads even when clearing persistent storage fails", () => {
    const reload = vi.fn();
    const removeItem = vi.fn(() => {
      throw new DOMException("blocked", "SecurityError");
    });
    clearStoredModelAndReload({ removeItem }, reload);
    expect(removeItem).toHaveBeenCalledWith(MODEL_STORAGE);
    expect(reload).toHaveBeenCalledOnce();
  });
});

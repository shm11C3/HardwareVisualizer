import { useEffect } from "react";

const DOCUMENT_HIDDEN_CLASS = "hardviz-document-hidden";

export const useDocumentVisibilityClass = (
  className = DOCUMENT_HIDDEN_CLASS,
) => {
  useEffect(() => {
    const root = document.documentElement;
    const syncVisibilityClass = () => {
      root.classList.toggle(className, document.hidden);
    };

    syncVisibilityClass();
    document.addEventListener("visibilitychange", syncVisibilityClass);

    return () => {
      document.removeEventListener("visibilitychange", syncVisibilityClass);
      root.classList.remove(className);
    };
  }, [className]);
};

/**
 * useAppKeyboardShortcuts - Keyboard shortcuts for view switching and chat toggle
 */

import { useEffect, useRef } from "react";
import { register, unregister } from "@tauri-apps/plugin-global-shortcut";
import type { ViewType } from "@/types/chat";

interface UseAppKeyboardShortcutsProps {
  currentView: ViewType;
  setCurrentView: (view: ViewType) => void;
  toggleChatVisible: (view: ViewType) => void;
  toggleReviewsPanel?: () => void;
  openProjectWizard?: () => void;
  hasProjects?: boolean;
  showWelcomeOverlay?: boolean;
  openWelcomeOverlay?: () => void;
  closeWelcomeOverlay?: () => void;
  welcomeOverlayReturnView?: ViewType | null;
}

export function useAppKeyboardShortcuts({
  currentView,
  setCurrentView,
  toggleChatVisible,
  toggleReviewsPanel,
  openProjectWizard,
  hasProjects,
  showWelcomeOverlay,
  openWelcomeOverlay,
  closeWelcomeOverlay,
  welcomeOverlayReturnView,
}: UseAppKeyboardShortcutsProps) {
  // Keyboard shortcuts for view switching (Cmd+1-5 for main views, Cmd+K for chat)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Escape to close welcome overlay (no modifier required)
      if (e.key === "Escape" && showWelcomeOverlay && closeWelcomeOverlay) {
        e.preventDefault();
        if (welcomeOverlayReturnView) {
          setCurrentView(welcomeOverlayReturnView);
        }
        closeWelcomeOverlay();
        return;
      }

      if (e.metaKey || e.ctrlKey) {
        switch (e.key) {
          case "1":
            e.preventDefault();
            setCurrentView("kanban");
            break;
          case "2":
            e.preventDefault();
            setCurrentView("graph");
            break;
          case "3":
            e.preventDefault();
            setCurrentView("ideation");
            break;
          case "4":
            e.preventDefault();
            setCurrentView("extensibility");
            break;
          case "5":
            e.preventDefault();
            setCurrentView("activity");
            break;
          case "6":
          case ".":
          case ",":
            // Cmd+6, Cmd+. or Cmd+, for settings (Cmd+, may not work in dev mode)
            e.preventDefault();
            setCurrentView("settings");
            break;
          case "k":
          case "K": {
            // Cmd+K to toggle chat panel (skip if in input/textarea or on ideation)
            if (currentView === "ideation") {
              return; // Ideation has built-in chat, no toggle needed
            }
            const activeElement = document.activeElement;
            if (
              activeElement instanceof HTMLInputElement ||
              activeElement instanceof HTMLTextAreaElement
            ) {
              return;
            }
            e.preventDefault();
            toggleChatVisible(currentView);
            break;
          }
          case "n":
          case "N": {
            // Cmd+Shift+N: Always open project wizard (global)
            // Cmd+N: Open project wizard only on welcome screen (no projects)
            if (!openProjectWizard) {
              return;
            }
            const activeEl = document.activeElement;
            if (
              activeEl instanceof HTMLInputElement ||
              activeEl instanceof HTMLTextAreaElement
            ) {
              return;
            }
            if (e.shiftKey) {
              // Cmd+Shift+N: Always available
              e.preventDefault();
              openProjectWizard();
            } else if (!hasProjects) {
              // Cmd+N: Only on welcome screen (no projects)
              e.preventDefault();
              openProjectWizard();
            }
            break;
          }
          case "w":
          case "W": {
            // Cmd+Shift+W: Toggle welcome screen overlay
            if (!e.shiftKey || !openWelcomeOverlay || !hasProjects) {
              return;
            }
            const activeEl = document.activeElement;
            if (
              activeEl instanceof HTMLInputElement ||
              activeEl instanceof HTMLTextAreaElement
            ) {
              return;
            }
            e.preventDefault();
            if (showWelcomeOverlay && closeWelcomeOverlay) {
              // Already showing - close it
              if (welcomeOverlayReturnView) {
                setCurrentView(welcomeOverlayReturnView);
              }
              closeWelcomeOverlay();
            } else {
              // Open welcome overlay
              openWelcomeOverlay();
            }
            break;
          }
          case "r":
          case "R": {
            // Cmd+Shift+R: Toggle reviews panel
            if (!e.shiftKey || !toggleReviewsPanel) {
              return;
            }
            const activeEl = document.activeElement;
            if (
              activeEl instanceof HTMLInputElement ||
              activeEl instanceof HTMLTextAreaElement
            ) {
              return;
            }
            e.preventDefault();
            toggleReviewsPanel();
            break;
          }
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [setCurrentView, toggleChatVisible, toggleReviewsPanel, currentView, openProjectWizard, hasProjects, showWelcomeOverlay, openWelcomeOverlay, closeWelcomeOverlay, welcomeOverlayReturnView]);

  // Global shortcut for Cmd+, (registered at OS level to bypass DevTools interception)
  const setCurrentViewRef = useRef(setCurrentView);

  useEffect(() => {
    setCurrentViewRef.current = setCurrentView;
  }, [setCurrentView]);

  useEffect(() => {
    const shortcut = "CommandOrControl+,";

    register(shortcut, () => {
      setCurrentViewRef.current("settings");
    }).catch(() => {
      // Ignore registration errors
    });

    return () => {
      unregister(shortcut).catch(() => {
        // Ignore unregister errors on cleanup
      });
    };
  }, []);
}

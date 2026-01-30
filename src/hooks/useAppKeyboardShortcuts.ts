/**
 * useAppKeyboardShortcuts - Keyboard shortcuts for view switching and chat toggle
 */

import { useEffect, useRef } from "react";
import { register, unregister } from "@tauri-apps/plugin-global-shortcut";
import type { ViewType } from "@/types/chat";

interface UseAppKeyboardShortcutsProps {
  currentView: ViewType;
  setCurrentView: (view: ViewType) => void;
  toggleChatPanel: () => void;
  toggleChatCollapsed: () => void;
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
  toggleChatPanel,
  toggleChatCollapsed,
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
            setCurrentView("ideation");
            break;
          case "3":
            e.preventDefault();
            setCurrentView("extensibility");
            break;
          case "4":
            e.preventDefault();
            setCurrentView("activity");
            break;
          case "5":
          case ".":
          case ",":
            // Cmd+5, Cmd+. or Cmd+, for settings (Cmd+, may not work in dev mode)
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
            // Use split layout toggle for kanban, floating panel toggle for other views
            if (currentView === "kanban") {
              toggleChatCollapsed();
            } else {
              toggleChatPanel();
            }
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
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [setCurrentView, toggleChatPanel, currentView, toggleChatCollapsed, openProjectWizard, hasProjects, showWelcomeOverlay, openWelcomeOverlay, closeWelcomeOverlay, welcomeOverlayReturnView]);

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

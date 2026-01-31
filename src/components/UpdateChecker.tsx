/**
 * UpdateChecker - Checks for app updates on mount and shows notification
 *
 * Uses tauri-plugin-updater to check GitHub releases for new versions.
 * Shows a toast notification with option to download and install.
 */

import { useEffect, useRef } from "react";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Download } from "lucide-react";

export function UpdateChecker() {
  const hasChecked = useRef(false);

  useEffect(() => {
    // Only check once per app session
    if (hasChecked.current) return;
    hasChecked.current = true;

    const checkForUpdates = async () => {
      try {
        const update = await check();
        if (update) {
          showUpdateNotification(update);
        }
      } catch (error) {
        // Silently fail - update check is non-critical
        // In development or when endpoint is not configured, this will fail
        console.debug("Update check failed:", error);
      }
    };

    // Delay check to avoid blocking app startup
    const timeoutId = setTimeout(checkForUpdates, 3000);
    return () => clearTimeout(timeoutId);
  }, []);

  return null;
}

function showUpdateNotification(update: Update) {
  toast(
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <Download className="h-4 w-4 text-[#ff6b35]" />
        <span className="font-medium">Update Available</span>
      </div>
      <p className="text-sm text-muted-foreground">
        Version {update.version} is ready to install.
      </p>
      <div className="flex gap-2 mt-1">
        <Button
          size="sm"
          variant="default"
          onClick={() => installUpdate(update)}
          className="h-7 px-3 text-xs"
          style={{ backgroundColor: "#ff6b35" }}
        >
          Update Now
        </Button>
        <Button
          size="sm"
          variant="ghost"
          onClick={() => toast.dismiss()}
          className="h-7 px-3 text-xs"
        >
          Later
        </Button>
      </div>
    </div>,
    {
      duration: Infinity,
      id: "update-available",
    }
  );
}

async function installUpdate(update: Update) {
  const toastId = "update-progress";

  toast.loading("Downloading update...", { id: toastId });

  try {
    let totalBytes = 0;
    let downloadedBytes = 0;

    await update.downloadAndInstall((progress) => {
      if (progress.event === "Started" && progress.data.contentLength) {
        totalBytes = progress.data.contentLength;
      } else if (progress.event === "Progress") {
        downloadedBytes += progress.data.chunkLength;
        if (totalBytes > 0) {
          const percent = Math.round((downloadedBytes / totalBytes) * 100);
          toast.loading(`Downloading update... ${percent}%`, { id: toastId });
        }
      } else if (progress.event === "Finished") {
        toast.loading("Installing update...", { id: toastId });
      }
    });

    toast.success("Update installed! Restarting...", { id: toastId });

    // Give user a moment to see the success message
    setTimeout(async () => {
      await relaunch();
    }, 1500);
  } catch (error) {
    toast.error("Failed to install update. Please try again later.", {
      id: toastId,
    });
    console.error("Update installation failed:", error);
  }
}

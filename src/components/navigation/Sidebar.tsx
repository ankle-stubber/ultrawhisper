import { useEffect, useState } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import {
  Activity,
  Zap,
  Package,
  MapPin,
  Clock,
  Settings,
  Circle,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { StoredWorkflow } from "../../lib/types";

interface SidebarProps {
  currentPage: string;
  onNavigate: (page: any) => void;
}

interface NavItem {
  id: string;
  label: string;
  icon: typeof Activity;
  showBadge?: boolean;
  badgeCount?: number;
}

export function Sidebar({ currentPage, onNavigate }: SidebarProps) {
  const [isRecording, setIsRecording] = useState(false);
  const [workflowCount, setWorkflowCount] = useState(0);
  const [overridesActive, setOverridesActive] = useState<string | null>(null);

  // Navigation items
  const navItems: NavItem[] = [
    { id: "monitor", label: "System Monitor", icon: Activity },
    { id: "workflows", label: "Workflows", icon: Zap, showBadge: true, badgeCount: workflowCount },
    { id: "models", label: "Models", icon: Package },
    { id: "destinations", label: "Destinations", icon: MapPin },
    { id: "history", label: "History", icon: Clock },
    { id: "settings", label: "Settings", icon: Settings },
  ];

  // Listen for recording events
  useEffect(() => {
    let micLevelTimeout: ReturnType<typeof setTimeout> | null = null;
    const cleanups: UnlistenFn[] = [];

    (async () => {
      // Recording indicator - use mic-level events as a proxy
      const unlistenMicLevel = await listen("mic-level", () => {
        setIsRecording(true);
        if (micLevelTimeout) clearTimeout(micLevelTimeout);
        micLevelTimeout = setTimeout(() => setIsRecording(false), 500);
      });
      cleanups.push(unlistenMicLevel);

      // Workflow count
      const unlistenWorkflows = await listen("workflows-changed", async () => {
        try {
          const workflows = await invoke<StoredWorkflow[]>("list_workflows");
          const enabledCount = workflows.filter((w) => w.enabled).length;
          setWorkflowCount(enabledCount);
        } catch (error) {
          console.error("Failed to get workflows:", error);
        }
      });
      cleanups.push(unlistenWorkflows);

      // Overrides active
      const unlistenOverrides = await listen<{ path: string; count: number }>(
        "overrides-active",
        (event) => {
          if (event.payload.count > 0) {
            setOverridesActive(`${event.payload.count} overrides active`);
          } else {
            setOverridesActive(null);
          }
        }
      );
      cleanups.push(unlistenOverrides);

      // Initial workflow count
      try {
        const workflows = await invoke<StoredWorkflow[]>("list_workflows");
        const enabledCount = workflows.filter((w) => w.enabled).length;
        setWorkflowCount(enabledCount);
      } catch (error) {
        console.error("Failed to get initial workflows:", error);
      }
    })();

    return () => {
      cleanups.forEach((fn) => fn());
      if (micLevelTimeout) clearTimeout(micLevelTimeout);
    };
  }, []);

  return (
    <div className="w-20 uw-bg-surface border-r uw-border-default flex flex-col">
      {/* Logo */}
      <div className="flex items-center justify-center h-16 border-b uw-border-default">
        <div className="w-10 h-10 bg-gradient-to-br from-green-500 to-green-600 rounded-lg flex items-center justify-center font-bold text-gray-950">
          U
        </div>
      </div>

      {/* Overrides Banner */}
      {overridesActive && (
        <div className="px-2 py-2 border-b uw-border-default">
          <div className="text-center">
            <div className="text-[10px] uw-text-warning font-medium">
              {overridesActive}
            </div>
          </div>
        </div>
      )}

      {/* Navigation Items */}
      <nav className="flex-1 p-2 space-y-1">
        {navItems.map((item) => {
          const Icon = item.icon;
          const isActive = currentPage === item.id;

          return (
            <button
              key={item.id}
              onClick={() => onNavigate(item.id)}
              className={`
                relative w-full aspect-square rounded-lg flex flex-col items-center justify-center
                transition-all duration-200 group
                ${isActive
                  ? "uw-bg-primary-dim uw-text-accent"
                  : "uw-text-secondary hover:uw-text-primary hover:bg-gray-800/50"
                }
              `}
              title={item.label}
            >
              <Icon className="w-5 h-5" />

              {/* Badge */}
              {item.showBadge && item.badgeCount !== undefined && item.badgeCount > 0 && (
                <span className="absolute -top-1 -right-1 uw-bg-primary text-gray-950 text-[10px] font-bold rounded-full min-w-[18px] h-[18px] flex items-center justify-center px-1">
                  {item.badgeCount}
                </span>
              )}

              {/* Tooltip */}
              <div className="absolute left-full ml-2 px-2 py-1 uw-bg-elevated uw-text-primary text-xs rounded-md whitespace-nowrap opacity-0 group-hover:opacity-100 pointer-events-none transition-opacity z-50">
                {item.label}
              </div>
            </button>
          );
        })}
      </nav>

      {/* Bottom Section */}
      <div className="border-t uw-border-default">
        {/* Recording Indicator Pill (Above Live Preview) */}
        {isRecording && (
          <div className="px-2 py-2 border-b uw-border-default">
            <div className="flex items-center justify-center">
              <div className="relative">
                <Circle className="w-6 h-6 uw-text-error animate-pulse" fill="currentColor" />
                <span className="absolute -bottom-1 left-1/2 transform -translate-x-1/2 text-[10px] uw-text-error font-medium">
                  REC
                </span>
              </div>
            </div>
          </div>
        )}

        {/* Live Transcription */}
        <div className="p-2">
          <button
            onClick={() => onNavigate("live")}
            className={`
              w-full aspect-square rounded-lg flex flex-col items-center justify-center
              transition-all duration-200 group
              ${currentPage === "live"
                ? "uw-bg-primary-dim uw-text-accent"
                : "uw-text-secondary hover:uw-text-primary hover:bg-gray-800/50"
              }
            `}
            title="Live Transcription (Preview)"
          >
            <Activity className="w-5 h-5" />
            <span className="text-[8px] mt-1 opacity-60">PREVIEW</span>

            {/* Tooltip */}
            <div className="absolute left-full ml-2 px-2 py-1 uw-bg-elevated uw-text-primary text-xs rounded-md whitespace-nowrap opacity-0 group-hover:opacity-100 pointer-events-none transition-opacity z-50 bottom-0">
              Live Transcription (Coming Soon)
            </div>
          </button>
        </div>
      </div>
    </div>
  );
}

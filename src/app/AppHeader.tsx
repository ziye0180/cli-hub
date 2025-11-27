import { CSSProperties, ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { ArrowLeft, Settings } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { View } from "./types";
import type { AppId } from "@/lib/api";

/**
 * AppHeader
 * 负责头部可拖拽区域、标题与导航按钮。右侧操作区由调用方传入，保持高内聚低耦合。
 */
interface AppHeaderProps {
  currentView: View;
  activeApp: AppId;
  onBackToProviders: () => void;
  onOpenSettings: () => void;
  rightActions: ReactNode;
  /**
   * 允许调用方在左侧品牌区域追加元素（例如 UpdateBadge），以保持原有视觉位置。
   */
  leftAddon?: ReactNode;
}

export function AppHeader({
  currentView,
  activeApp,
  onBackToProviders,
  onOpenSettings,
  rightActions,
  leftAddon,
}: AppHeaderProps) {
  const { t } = useTranslation();

  return (
    <header
      className="glass-header fixed top-0 z-50 w-full py-3 transition-all duration-300"
      data-tauri-drag-region
      style={{ WebkitAppRegion: "drag" } as CSSProperties}
    >
      <div className="h-4 w-full" aria-hidden data-tauri-drag-region />
      <div
        className="mx-auto max-w-[56rem] px-6 flex flex-wrap items-center justify-between gap-2"
        data-tauri-drag-region
        style={{ WebkitAppRegion: "drag" } as CSSProperties}
      >
        <div
          className="flex items-center gap-1"
          style={{ WebkitAppRegion: "no-drag" } as CSSProperties}
        >
          {currentView !== "providers" ? (
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="icon"
                onClick={onBackToProviders}
                className="mr-2 rounded-lg"
              >
                <ArrowLeft className="h-4 w-4" />
              </Button>
              <h1 className="text-lg font-semibold">
                {currentView === "settings" && t("settings.title")}
                {currentView === "prompts" &&
                  t("prompts.title", { appName: t(`apps.${activeApp}`) })}
                {currentView === "skills" && t("skills.title")}
                {currentView === "mcp" && t("mcp.unifiedPanel.title")}
                {currentView === "agents" && t("agents.manage")}
              </h1>
            </div>
          ) : (
            <>
              <div className="flex items-center gap-2">
                <a
                  href="https://github.com/ziye0180/cli-hub"
                  target="_blank"
                  rel="noreferrer"
                  className="text-xl font-semibold text-blue-500 transition-colors hover:text-blue-600 dark:text-blue-400 dark:hover:text-blue-300"
                >
                  CLI Hub
                </a>
                <div className="h-5 w-[1px] bg-black/10 dark:bg-white/15" />
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={onOpenSettings}
                  title={t("common.settings")}
                  className="hover:bg-black/5 dark:hover:bg-white/5"
                >
                  <Settings className="h-4 w-4" />
                </Button>
              </div>
              {leftAddon}
            </>
          )}
        </div>

        <div
          className="flex items-center gap-2"
          style={{ WebkitAppRegion: "no-drag" } as CSSProperties}
        >
          {rightActions}
        </div>
      </div>
    </header>
  );
}

import { RefObject } from "react";
import { Plus, RefreshCw, Wrench, Server, Book, Bot } from "lucide-react";
import { Button } from "@/components/ui/button";
import { AppSwitcher } from "@/components/AppSwitcher";
import type { View } from "./types";
import type { AppId } from "@/lib/api";
import { useTranslation } from "react-i18next";

/**
 * ViewActions
 * 集中管理头部右侧的操作按钮，根据 currentView 动态展示不同的操作。
 * 这样主入口组件无需维护大量条件渲染，便于后续扩展新的视图。
 */
interface ViewActionsProps {
  currentView: View;
  activeApp: AppId;
  isClaudeApp: boolean;
  onSetCurrentView: (view: View) => void;
  onAddProvider: () => void;
  onOpenPromptsAdd: () => void;
  onOpenMcpAdd: () => void;
  onSkillsRefresh: () => void;
  onOpenSkillRepoManager: () => void;
  onSwitchApp: (app: AppId) => void;
  promptPanelRef: RefObject<any>;
  mcpPanelRef: RefObject<any>;
  skillsPageRef: RefObject<any>;
}

export function ViewActions({
  currentView,
  activeApp,
  isClaudeApp,
  onSetCurrentView,
  onAddProvider,
  onOpenPromptsAdd,
  onOpenMcpAdd,
  onSkillsRefresh,
  onOpenSkillRepoManager,
  onSwitchApp,
  promptPanelRef,
  mcpPanelRef,
  skillsPageRef,
}: ViewActionsProps) {
  const { t } = useTranslation();

  // 视图专属的按钮分支，保持与原 UI 行为一致。
  if (currentView === "prompts") {
    return (
      <Button
        onClick={() => {
          onOpenPromptsAdd();
          promptPanelRef.current?.openAdd();
        }}
        className="bg-orange-500 hover:bg-orange-600 dark:bg-orange-500 dark:hover:bg-orange-600 text-white shadow-lg shadow-orange-500/30 dark:shadow-orange-500/40"
        title={t("prompts.add")}
      >
        <Plus className="h-4 w-4 mr-1.5" />
        {t("prompts.add")}
      </Button>
    );
  }

  if (currentView === "mcp") {
    return (
      <Button
        onClick={() => {
          onOpenMcpAdd();
          mcpPanelRef.current?.openAdd();
        }}
        className="bg-orange-500 hover:bg-orange-600 dark:bg-orange-500 dark:hover:bg-orange-600 text-white shadow-lg shadow-orange-500/30 dark:shadow-orange-500/40"
        title={t("mcp.unifiedPanel.addServer")}
      >
        <Plus className="h-4 w-4 mr-1.5" />
        {t("mcp.unifiedPanel.addServer")}
      </Button>
    );
  }

  if (currentView === "skills") {
    return (
      <div className="flex items-center gap-2">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            onSkillsRefresh();
            skillsPageRef.current?.refresh();
          }}
          className="hover:bg-black/5 dark:hover:bg-white/5"
        >
          <RefreshCw className="h-4 w-4 mr-2" />
          {t("skills.refresh")}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => {
            onOpenSkillRepoManager();
            skillsPageRef.current?.openRepoManager();
          }}
          className="hover:bg-black/5 dark:hover:bg-white/5"
        >
          <Wrench className="h-4 w-4 mr-2" />
          {t("skills.repoManager")}
        </Button>
      </div>
    );
  }

  // 默认为 providers 视图，展示切换器与管理入口。
  return (
    <div className="flex items-center gap-2">
      <AppSwitcher activeApp={activeApp} onSwitch={onSwitchApp} />

      <div className="h-8 w-[1px] bg-black/10 dark:bg-white/10 mx-1" />

      <div className="glass p-1 rounded-xl flex items-center gap-1">
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onSetCurrentView("prompts")}
          className="text-muted-foreground hover:text-foreground hover:bg-black/5 dark:hover:bg-white/5"
          title={t("prompts.manage")}
        >
          <Book className="h-4 w-4" />
          <span className="ml-1.5">{t("prompts.manage")}</span>
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onSetCurrentView("skills")}
          className={`text-muted-foreground hover:text-foreground hover:bg-black/5 dark:hover:bg-white/5 ${!isClaudeApp ? "opacity-0 pointer-events-none" : ""}`}
          title={t("skills.manage")}
          disabled={!isClaudeApp}
          tabIndex={!isClaudeApp ? -1 : undefined}
        >
          <Wrench className="h-4 w-4" />
          <span className="ml-1.5">{t("skills.manage")}</span>
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onSetCurrentView("mcp")}
          className="text-muted-foreground hover:text-foreground hover:bg-black/5 dark:hover:bg-white/5"
          title="MCP"
        >
          <Server className="h-4 w-4" />
          <span className="ml-1.5">MCP</span>
        </Button>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => onSetCurrentView("agents")}
          className={`text-muted-foreground hover:text-foreground hover:bg-black/5 dark:hover:bg-white/5 ${!isClaudeApp ? "opacity-0 pointer-events-none" : ""}`}
          title={t("agents.manage")}
          disabled={!isClaudeApp}
          tabIndex={!isClaudeApp ? -1 : undefined}
        >
          <Bot className="h-4 w-4" />
          <span className="ml-1.5">{t("agents.manage")}</span>
        </Button>
      </div>

      <Button
        onClick={onAddProvider}
        className="ml-2 bg-orange-500 hover:bg-orange-600 dark:bg-orange-500 dark:hover:bg-orange-600 text-white shadow-lg shadow-orange-500/30 dark:shadow-orange-500/40"
      >
        <Plus className="h-4 w-4 mr-1.5" />
        {t("provider.addProvider")}
      </Button>
    </div>
  );
}

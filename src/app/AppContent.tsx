import { RefObject } from "react";
import type { View } from "./types";
import type { Provider } from "@/types";
import type { AppId } from "@/lib/api";
import { ProviderList } from "@/components/providers/ProviderList";
import { SettingsPage } from "@/components/settings/SettingsPage";
import PromptPanel from "@/components/prompts/PromptPanel";
import UnifiedMcpPanel from "@/components/mcp/UnifiedMcpPanel";
import { SkillsPage } from "@/components/skills/SkillsPage";
import { AgentsPanel } from "@/components/agents/AgentsPanel";

/**
 * AppContent
 * 负责根据当前视图切换展示不同的主内容区域，所有数据与回调均从父组件传入。
 * 这样可以让 App.tsx 聚焦于状态管理，不再承载冗长的 JSX 分支。
 */
interface AppContentProps {
  currentView: View;
  activeApp: AppId;
  providers: Record<string, Provider>;
  currentProviderId: string;
  isLoading: boolean;
  onSwitchProvider: (provider: Provider) => Promise<void> | void;
  onEditProvider: (provider: Provider) => void;
  onDeleteProvider: (provider: Provider) => void;
  onDuplicateProvider: (provider: Provider) => Promise<void> | void;
  onConfigureUsage: (provider: Provider) => void;
  onOpenWebsite: (url: string) => Promise<void> | void;
  onImportSuccess: () => Promise<void> | void;
  setCurrentView: (view: View) => void;
  promptPanelRef: RefObject<any>;
  mcpPanelRef: RefObject<any>;
  skillsPageRef: RefObject<any>;
}

export function AppContent({
  currentView,
  activeApp,
  providers,
  currentProviderId,
  isLoading,
  onSwitchProvider,
  onEditProvider,
  onDeleteProvider,
  onDuplicateProvider,
  onConfigureUsage,
  onOpenWebsite,
  onImportSuccess,
  setCurrentView,
  promptPanelRef,
  mcpPanelRef,
  skillsPageRef,
}: AppContentProps) {
  switch (currentView) {
    case "settings":
      return (
        <SettingsPage
          open={true}
          onOpenChange={() => setCurrentView("providers")}
          onImportSuccess={onImportSuccess}
        />
      );
    case "prompts":
      return (
        <PromptPanel
          ref={promptPanelRef}
          open={true}
          onOpenChange={() => setCurrentView("providers")}
          appId={activeApp}
        />
      );
    case "skills":
      return (
        <SkillsPage
          ref={skillsPageRef}
          onClose={() => setCurrentView("providers")}
        />
      );
    case "mcp":
      return (
        <UnifiedMcpPanel
          ref={mcpPanelRef}
          onOpenChange={() => setCurrentView("providers")}
        />
      );
    case "agents":
      return <AgentsPanel onOpenChange={() => setCurrentView("providers")}/>;
    default:
      return (
        <div className="mx-auto max-w-[56rem] px-6 mt-4 space-y-4">
          <ProviderList
            providers={providers}
            currentProviderId={currentProviderId}
            appId={activeApp}
            isLoading={isLoading}
            onSwitch={onSwitchProvider}
            onEdit={onEditProvider}
            onDelete={onDeleteProvider}
            onDuplicate={onDuplicateProvider}
            onConfigureUsage={onConfigureUsage}
            onOpenWebsite={onOpenWebsite}
          />
        </div>
      );
  }
}

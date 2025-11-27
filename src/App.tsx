import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  providersApi,
  settingsApi,
  type AppId,
  type ProviderSwitchEvent,
} from "@/lib/api";
import { useProvidersQuery } from "@/lib/query";
import type { Provider } from "@/types";
import { useProviderActions } from "@/hooks/useProviderActions";
import { extractErrorMessage } from "@/utils/errorUtils";
import { UpdateBadge } from "@/components/UpdateBadge";
import { AppShell } from "@/app/AppShell";
import { AppHeader } from "@/app/AppHeader";
import { ViewActions } from "@/app/ViewActions";
import { AppContent } from "@/app/AppContent";
import { EnvConflictBanner } from "@/app/EnvConflictBanner";
import { ProvidersModals } from "@/app/modals/ProvidersModals";
import { useEnvConflicts } from "@/app/hooks/useEnvConflicts";
import type { View } from "@/app/types";

/**
 * App 入口
 * 使命：保留核心状态与业务动作，将 UI 分支和副作用拆分到独立组件/Hook，
 * 让文件保持可读、可维护，同时保持功能不变。
 */
function App() {
  const { t } = useTranslation();

  // -------------------- 视图与应用状态 --------------------
  const [activeApp, setActiveApp] = useState<AppId>("claude");
  const [currentView, setCurrentView] = useState<View>("providers");
  const [isAddOpen, setIsAddOpen] = useState(false);

  // -------------------- Provider 相关状态 --------------------
  const [editingProvider, setEditingProvider] = useState<Provider | null>(null);
  const [usageProvider, setUsageProvider] = useState<Provider | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<Provider | null>(null);

  // -------------------- Env 冲突状态通过 Hook 管理 --------------------
  const {
    envConflicts,
    showEnvBanner,
    handleDismissBanner,
    handleRecheckAfterDelete,
  } = useEnvConflicts(activeApp);

  // -------------------- 引用，供子组件触发内部方法 --------------------
  const promptPanelRef = useRef<any>(null);
  const mcpPanelRef = useRef<any>(null);
  const skillsPageRef = useRef<any>(null);

  // -------------------- 数据获取 --------------------
  const { data, isLoading, refetch } = useProvidersQuery(activeApp);
  const providers = useMemo(() => data?.providers ?? {}, [data]);
  const currentProviderId = data?.currentProviderId ?? "";
  const isClaudeApp = activeApp === "claude";

  // 🎯 使用 useProviderActions Hook 统一管理所有 Provider 操作
  const {
    addProvider,
    updateProvider,
    switchProvider,
    deleteProvider,
    saveUsageScript,
  } = useProviderActions(activeApp);

  // 监听来自托盘菜单的切换事件；当托盘切换到当前 app 时刷新列表。
  useEffect(() => {
    let unsubscribe: (() => void) | undefined;

    const setupListener = async () => {
      try {
        unsubscribe = await providersApi.onSwitched(
          async (event: ProviderSwitchEvent) => {
            if (event.appType === activeApp) {
              await refetch();
            }
          },
        );
      } catch (error) {
        console.error("[App] Failed to subscribe provider switch event", error);
      }
    };

    setupListener();
    return () => {
      unsubscribe?.();
    };
  }, [activeApp, refetch]);

  // 打开网站链接，保持原有错误提示体验。
  const handleOpenWebsite = async (url: string) => {
    try {
      await settingsApi.openExternal(url);
    } catch (error) {
      const detail =
        extractErrorMessage(error) ||
        t("notifications.openLinkFailed", {
          defaultValue: "链接打开失败",
        });
      toast.error(detail);
    }
  };

  // 编辑供应商
  const handleEditProvider = async (provider: Provider) => {
    await updateProvider(provider);
    setEditingProvider(null);
  };

  // 确认删除供应商
  const handleConfirmDelete = async () => {
    if (!confirmDelete) return;
    await deleteProvider(confirmDelete.id);
    setConfirmDelete(null);
  };

  // 复制供应商：保持 sortIndex 逻辑与原行为一致。
  const handleDuplicateProvider = async (provider: Provider) => {
    const newSortIndex =
      provider.sortIndex !== undefined ? provider.sortIndex + 1 : undefined;

    const duplicatedProvider: Omit<Provider, "id" | "createdAt"> = {
      name: `${provider.name} copy`,
      settingsConfig: JSON.parse(JSON.stringify(provider.settingsConfig)),
      websiteUrl: provider.websiteUrl,
      category: provider.category,
      sortIndex: newSortIndex,
      meta: provider.meta
        ? JSON.parse(JSON.stringify(provider.meta))
        : undefined,
    };

    // sortIndex 冲突处理：为插入位置腾出空间。
    if (provider.sortIndex !== undefined) {
      const updates = Object.values(providers)
        .filter(
          (p) =>
            p.sortIndex !== undefined &&
            p.sortIndex >= newSortIndex! &&
            p.id !== provider.id,
        )
        .map((p) => ({
          id: p.id,
          sortIndex: p.sortIndex! + 1,
        }));

      if (updates.length > 0) {
        try {
          await providersApi.updateSortOrder(updates, activeApp);
        } catch (error) {
          console.error("[App] Failed to update sort order", error);
          toast.error(
            t("provider.sortUpdateFailed", {
              defaultValue: "排序更新失败",
            }),
          );
          return; // 排序失败则不继续新增
        }
      }
    }

    await addProvider(duplicatedProvider);
  };

  // 导入配置成功后刷新列表与托盘菜单（保持与原逻辑一致）。
  const handleImportSuccess = async () => {
    await refetch();
    try {
      await providersApi.updateTrayMenu();
    } catch (error) {
      console.error("[App] Failed to refresh tray menu", error);
    }
  };

  // 根据当前待删除项生成确认框文案，避免在弹窗组件内耦合翻译逻辑。
  const confirmDialogTitle = t("confirm.deleteProvider");
  const confirmDialogMessage = confirmDelete
    ? t("confirm.deleteProviderMessage", { name: confirmDelete.name })
    : "";

  return (
    <AppShell
      header={
        <>
          {/* 顶部 4px 拖拽区域，确保窗口顶部可拖动 */}
          <div
            className="fixed top-0 left-0 right-0 h-4 z-[60]"
            data-tauri-drag-region
            style={{ WebkitAppRegion: "drag" } as any}
          />

          <AppHeader
            currentView={currentView}
            activeApp={activeApp}
            onBackToProviders={() => setCurrentView("providers")}
            onOpenSettings={() => setCurrentView("settings")}
            leftAddon={
              currentView === "providers" ? (
                <UpdateBadge onClick={() => setCurrentView("settings")}/>
              ) : null
            }
            rightActions={
              <ViewActions
                currentView={currentView}
                activeApp={activeApp}
                isClaudeApp={isClaudeApp}
                onSetCurrentView={setCurrentView}
                onAddProvider={() => setIsAddOpen(true)}
                onOpenPromptsAdd={() => {}}
                onOpenMcpAdd={() => {}}
                onSkillsRefresh={() => {}}
                onOpenSkillRepoManager={() => {}}
                onSwitchApp={setActiveApp}
                promptPanelRef={promptPanelRef}
                mcpPanelRef={mcpPanelRef}
                skillsPageRef={skillsPageRef}
              />
            }
          />
        </>
      }
    >
      {/* 横幅放在 Header 下方，保持原有层级关系 */}
      <EnvConflictBanner
        showEnvBanner={showEnvBanner}
        envConflicts={envConflicts}
        onDismiss={handleDismissBanner}
        onDeleted={handleRecheckAfterDelete}
      />

      <main
        className={`flex-1 overflow-y-auto pb-12 animate-fade-in scroll-overlay ${
          currentView === "providers" ? "pt-32" : "pt-24"
        }`}
        style={{ overflowX: "hidden" }}
      >
        <AppContent
          currentView={currentView}
          activeApp={activeApp}
          providers={providers}
          currentProviderId={currentProviderId}
          isLoading={isLoading}
          onSwitchProvider={switchProvider}
          onEditProvider={setEditingProvider}
          onDeleteProvider={setConfirmDelete}
          onDuplicateProvider={handleDuplicateProvider}
          onConfigureUsage={setUsageProvider}
          onOpenWebsite={handleOpenWebsite}
          onImportSuccess={handleImportSuccess}
          setCurrentView={setCurrentView}
          promptPanelRef={promptPanelRef}
          mcpPanelRef={mcpPanelRef}
          skillsPageRef={skillsPageRef}
        />
      </main>

      <ProvidersModals
        appId={activeApp}
        isAddOpen={isAddOpen}
        setIsAddOpen={setIsAddOpen}
        editingProvider={editingProvider}
        setEditingProvider={setEditingProvider}
        usageProvider={usageProvider}
        setUsageProvider={setUsageProvider}
        confirmDelete={confirmDelete}
        setConfirmDelete={setConfirmDelete}
        onAddProvider={addProvider}
        onEditProvider={handleEditProvider}
        onDeleteConfirmed={handleConfirmDelete}
        onSaveUsageScript={saveUsageScript}
        confirmDialogTitle={confirmDialogTitle}
        confirmDialogMessage={confirmDialogMessage}
      />
    </AppShell>
  );
}

export default App;

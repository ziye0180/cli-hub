import { Dispatch, SetStateAction } from "react";
import type { Provider, UsageScript } from "@/types";
import type { AppId } from "@/lib/api";
import { AddProviderDialog } from "@/components/providers/AddProviderDialog";
import { EditProviderDialog } from "@/components/providers/EditProviderDialog";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import UsageScriptModal from "@/components/UsageScriptModal";
import { DeepLinkImportDialog } from "@/components/DeepLinkImportDialog";

/**
 * ProvidersModals
 * 将与 Provider 相关的弹窗集中管理，减少 App.tsx 的模板代码并保持状态显式传递。
 */
interface ProvidersModalsProps {
  appId: AppId;
  isAddOpen: boolean;
  setIsAddOpen: Dispatch<SetStateAction<boolean>>;
  editingProvider: Provider | null;
  setEditingProvider: Dispatch<SetStateAction<Provider | null>>;
  usageProvider: Provider | null;
  setUsageProvider: Dispatch<SetStateAction<Provider | null>>;
  confirmDelete: Provider | null;
  setConfirmDelete: Dispatch<SetStateAction<Provider | null>>;
  onAddProvider: (provider: Omit<Provider, "id" | "createdAt">) => Promise<void> | void;
  onEditProvider: (provider: Provider) => Promise<void> | void;
  onDeleteConfirmed: () => Promise<void> | void;
  onSaveUsageScript: (provider: Provider, script: UsageScript) => Promise<void> | void;
  confirmDialogTitle: string;
  confirmDialogMessage: string;
}

export function ProvidersModals({
  appId,
  isAddOpen,
  setIsAddOpen,
  editingProvider,
  setEditingProvider,
  usageProvider,
  setUsageProvider,
  confirmDelete,
  setConfirmDelete,
  onAddProvider,
  onEditProvider,
  onDeleteConfirmed,
  onSaveUsageScript,
  confirmDialogTitle,
  confirmDialogMessage,
}: ProvidersModalsProps) {
  return (
    <>
      <AddProviderDialog
        open={isAddOpen}
        onOpenChange={setIsAddOpen}
        appId={appId}
        onSubmit={onAddProvider}
      />

      <EditProviderDialog
        open={Boolean(editingProvider)}
        provider={editingProvider}
        onOpenChange={(open) => {
          if (!open) {
            setEditingProvider(null);
          }
        }}
        onSubmit={onEditProvider}
        appId={appId}
      />

      {usageProvider && (
        <UsageScriptModal
          provider={usageProvider}
          appId={appId}
          isOpen={Boolean(usageProvider)}
          onClose={() => setUsageProvider(null)}
          onSave={(script) => {
            void onSaveUsageScript(usageProvider, script);
          }}
        />
      )}

      <ConfirmDialog
        isOpen={Boolean(confirmDelete)}
        title={confirmDialogTitle}
        message={confirmDialogMessage}
        onConfirm={() => void onDeleteConfirmed()}
        onCancel={() => setConfirmDelete(null)}
      />

      <DeepLinkImportDialog />
    </>
  );
}

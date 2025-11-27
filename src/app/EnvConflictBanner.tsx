import type { EnvConflict } from "@/types/env";
import { EnvWarningBanner } from "@/components/env/EnvWarningBanner";

/**
 * EnvConflictBanner
 * 对现有 EnvWarningBanner 做轻量包装，聚合展示/关闭/删除后的复查逻辑，
 * 以便在 App 中直接复用并保持副作用可测试、易读。
 */
interface EnvConflictBannerProps {
  showEnvBanner: boolean;
  envConflicts: EnvConflict[];
  onDismiss: () => void;
  onDeleted: () => Promise<void> | void;
}

export function EnvConflictBanner({
  showEnvBanner,
  envConflicts,
  onDismiss,
  onDeleted,
}: EnvConflictBannerProps) {
  if (!showEnvBanner || envConflicts.length === 0) return null;

  return (
    <EnvWarningBanner
      conflicts={envConflicts}
      onDismiss={onDismiss}
      onDeleted={onDeleted}
    />
  );
}

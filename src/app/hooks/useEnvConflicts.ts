import { useEffect, useState } from "react";
import type { EnvConflict } from "@/types/env";
import { checkAllEnvConflicts, checkEnvConflicts } from "@/lib/api/env";
import type { AppId } from "@/lib/api";

/**
 * useEnvConflicts
 * 统一处理环境变量冲突的检测与展示状态，避免在 App 组件中充斥副作用细节。
 * 设计思路：
 * - 启动时：检查所有应用的冲突列表，决定是否展示横幅。
 * - 切换 activeApp 时：仅检查当前应用，增量合并到已有冲突，避免重复条目。
 * - 提供对外的显示控制方法（dismiss / afterDelete），便于横幅组件复用。
 */
export function useEnvConflicts(activeApp: AppId) {
  // 当前聚合的冲突列表，可能来自“全量启动检查”或“切换检查”。
  const [envConflicts, setEnvConflicts] = useState<EnvConflict[]>([]);
  // 控制横幅是否显示；与 sessionStorage 的开关保持一致，避免重复打扰用户。
  const [showEnvBanner, setShowEnvBanner] = useState(false);

  // 启动时检测所有应用的冲突，结果用于初始化横幅。
  useEffect(() => {
    const checkEnvOnStartup = async () => {
      try {
        const allConflicts = await checkAllEnvConflicts();
        const flatConflicts = Object.values(allConflicts).flat();

        if (flatConflicts.length > 0) {
          setEnvConflicts(flatConflicts);
          const dismissed = sessionStorage.getItem("env_banner_dismissed");
          if (!dismissed) {
            setShowEnvBanner(true);
          }
        }
      } catch (error) {
        console.error("[EnvConflicts] Failed to check environment conflicts on startup:", error);
      }
    };

    checkEnvOnStartup();
  }, []);

  // 切换 activeApp 时只检测当前应用，增量合并新发现的冲突。
  useEffect(() => {
    const checkEnvOnSwitch = async () => {
      try {
        const conflicts = await checkEnvConflicts(activeApp);

        if (conflicts.length > 0) {
          // 通过唯一键去重，避免在切换时重复展示同一条冲突。
          setEnvConflicts((prev) => {
            const existingKeys = new Set(prev.map((c) => `${c.varName}:${c.sourcePath}`));
            const newConflicts = conflicts.filter((c) => !existingKeys.has(`${c.varName}:${c.sourcePath}`));
            return [...prev, ...newConflicts];
          });

          const dismissed = sessionStorage.getItem("env_banner_dismissed");
          if (!dismissed) {
            setShowEnvBanner(true);
          }
        }
      } catch (error) {
        console.error("[EnvConflicts] Failed to check environment conflicts on app switch:", error);
      }
    };

    checkEnvOnSwitch();
  }, [activeApp]);

  /**
   * 用户主动关闭横幅时调用。同步写入 sessionStorage，保持与旧行为一致。
   */
  const handleDismissBanner = () => {
    setShowEnvBanner(false);
    sessionStorage.setItem("env_banner_dismissed", "true");
  };

  /**
   * 删除冲突项后重新检测全量冲突，用于横幅的 onDeleted 回调，确保最新状态。
   */
  const handleRecheckAfterDelete = async () => {
    try {
      const allConflicts = await checkAllEnvConflicts();
      const flatConflicts = Object.values(allConflicts).flat();
      setEnvConflicts(flatConflicts);
      if (flatConflicts.length === 0) {
        setShowEnvBanner(false);
      }
    } catch (error) {
      console.error("[EnvConflicts] Failed to re-check conflicts after deletion:", error);
    }
  };

  return {
    envConflicts,
    showEnvBanner,
    setShowEnvBanner,
    handleDismissBanner,
    handleRecheckAfterDelete,
  } as const;
}

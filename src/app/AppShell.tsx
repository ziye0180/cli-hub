import { ReactNode, CSSProperties } from "react";

/**
 * AppShell
 * 负责全局布局框架：背景色、文本色、最小高度等，与业务逻辑解耦。
 * 将拖拽区域、Header、Main 作为 children 组合，方便日后替换或扩展。
 */
interface AppShellProps {
  header: ReactNode;
  children: ReactNode;
}

export function AppShell({ header, children }: AppShellProps) {
  return (
    <div
      className="flex min-h-screen flex-col bg-background text-foreground selection:bg-primary/30"
      style={{ overflowX: "hidden" } as CSSProperties}
    >
      {header}
      {children}
    </div>
  );
}

// View 类型集中声明，便于在多文件间共享并保持枚举一致性。
// 如果后续新增视图，只需在这里扩展类型并同步使用它的组件即可。
export type View = "providers" | "settings" | "prompts" | "skills" | "mcp" | "agents";

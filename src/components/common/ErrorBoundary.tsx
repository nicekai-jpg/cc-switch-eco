import React, { Component, type ErrorInfo, type ReactNode } from "react";

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
    error: null,
  };

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error("Uncaught error inside ErrorBoundary:", error, errorInfo);
  }

  public render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }
      return (
        <div className="p-6 rounded-lg border border-destructive bg-destructive/10 text-destructive space-y-4">
          <div className="space-y-2">
            <h3 className="font-bold text-lg">组件渲染出错 (Render Error)</h3>
            <p className="text-sm">
              渲染该供应商的配置表单时发生了未捕获的错误。这通常是由于特定的配置项格式不兼容导致的。
            </p>
            <p className="text-sm font-semibold">
              错误信息: {this.state.error?.message || "未知错误"}
            </p>
          </div>
          {this.state.error?.stack && (
            <div className="space-y-1">
              <span className="text-xs font-semibold text-muted-foreground">错误堆栈:</span>
              <pre className="text-xs max-h-60 overflow-auto bg-black/20 p-3 rounded font-mono whitespace-pre-wrap break-all">
                {this.state.error.stack}
              </pre>
            </div>
          )}
        </div>
      );
    }

    return this.props.children;
  }
}

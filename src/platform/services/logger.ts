// 平台统一日志服务。
//
// 业务代码（包括各功能模块）统一使用 `logger.info/warn/error/debug`，
// 日志由 Rust 端写入 `<data>/log/mysystem-YYYY-MM-DD.log`，同时输出到终端。
// 模块中可直接 `import { logger } from '../../platform/services/logger'` 使用。

import { invoke } from '@tauri-apps/api/core'

export type LogLevel = 'debug' | 'info' | 'warn' | 'error'

/** 日志文件信息（与 Rust `LogFileInfo` 对应） */
export interface LogFileInfo {
  name: string
  size: number
  modifiedAt: number
}

function formatArg(arg: unknown): string {
  if (typeof arg === 'string') return arg
  if (arg instanceof Error) return arg.stack || `${arg.name}: ${arg.message}`
  try {
    return JSON.stringify(arg)
  } catch {
    return String(arg)
  }
}

function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
}

function write(level: LogLevel, args: unknown[]): void {
  const message = args.map(formatArg).join(' ')

  // 终端/控制台输出（打包后不可见，仅便于开发调试）
  if (level === 'error') {
    console.error(message)
  } else if (level === 'warn') {
    console.warn(message)
  } else if (level === 'debug') {
    console.debug(message)
  } else {
    console.log(message)
  }

  // 写入日志文件；浏览器环境（非 Tauri）下忽略
  if (!isTauri()) return
  invoke('write_log', { level, message }).catch(() => {
    // 日志写入失败不应影响业务
  })
}

export const logger = {
  debug(...args: unknown[]): void {
    write('debug', args)
  },
  info(...args: unknown[]): void {
    write('info', args)
  },
  warn(...args: unknown[]): void {
    write('warn', args)
  },
  error(...args: unknown[]): void {
    write('error', args)
  },
}

/** 列出日志文件（按修改时间倒序） */
export function listLogFiles(): Promise<LogFileInfo[]> {
  return invoke<LogFileInfo[]>('list_log_files')
}

/** 读取日志文件内容（最多返回最后 512 KB） */
export function readLogFile(name: string): Promise<string> {
  return invoke<string>('read_log_file', { name })
}

/** 获取日志目录的绝对路径 */
export function getLogDir(): Promise<string> {
  return invoke<string>('get_log_dir')
}

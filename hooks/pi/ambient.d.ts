/**
 * Ambient type declarations for the Pi extension runtime.
 *
 * These provide the minimal surface that hooks/pi/index.ts needs
 * so that the TypeScript LSP can type-check the reference hook
 * in this workspace.  The actual runtime types are provided by
 * @earendil-works/pi-coding-agent when the hook is installed inside Pi.
 */

declare module "@earendil-works/pi-coding-agent" {
	export interface ExtensionAPI {
		on(event: "session_start", handler: SessionStartHandler): void;
		on(event: "tool_result", handler: ToolResultHandler): void;
		exec(
			command: string,
			args: string[],
			options?: ExecOptions,
		): Promise<ExecResult>;
	}

	export type SessionStartHandler = (
		event: SessionStartEvent,
		ctx: ExtensionContext,
	) => void | Promise<void>;

	export type ToolResultHandler = (
		event: ToolResultEvent,
		ctx: ExtensionContext,
	) => undefined | Promise<undefined>;

	export interface SessionStartEvent {
		reason: string;
		previousSessionFile?: string;
	}

	export interface ToolResultEvent {
		toolName: string;
		toolCallId: string;
		input: Record<string, unknown>;
		content: unknown;
		details: unknown;
		isError: boolean;
	}

	export interface ExtensionContext {
		// intentionally minimal — the hook only uses ctx from the signature
	}

	export interface ExecOptions {
		input?: string;
		timeout?: number;
		signal?: AbortSignal;
	}

	export interface ExecResult {
		stdout: string;
		stderr: string;
		code: number | null;
		killed: boolean;
	}
}

import { z } from "zod";

const LogLevelSchema = z.enum(["debug", "info", "warn", "error", "silent"]);
export type LogLevel = z.infer<typeof LogLevelSchema>;

const parsedLevel = LogLevelSchema.safeParse(import.meta.env.VITE_LOG_LEVEL);

export const config = {
	logLevel: parsedLevel.success
		? parsedLevel.data
		: import.meta.env.DEV
			? "debug"
			: "warn",
} as const satisfies { logLevel: LogLevel };

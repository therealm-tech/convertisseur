import { config, type LogLevel } from "@/config";

type Fields = Record<string, unknown>;

const order: Record<LogLevel, number> = {
	debug: 10,
	info: 20,
	warn: 30,
	error: 40,
	silent: 100,
};

function emit(
	level: Exclude<LogLevel, "silent">,
	event: string,
	fields: Fields,
): void {
	if (order[level] < order[config.logLevel]) {
		return;
	}
	// biome-ignore lint/suspicious/noConsole: this module is the single sink for diagnostics.
	console[level](event, fields);
}

export const logger = {
	debug: (event: string, fields: Fields = {}) => emit("debug", event, fields),
	info: (event: string, fields: Fields = {}) => emit("info", event, fields),
	warn: (event: string, fields: Fields = {}) => emit("warn", event, fields),
	error: (event: string, fields: Fields = {}) => emit("error", event, fields),
};

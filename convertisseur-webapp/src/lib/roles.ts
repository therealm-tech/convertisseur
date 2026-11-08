import type { Role, User } from "@/lib/api";

export const roleLabels: Record<Role, string> = {
	user: "user",
	admin: "admin",
	super_admin: "super admin",
};

const rank: Record<Role, number> = { user: 0, admin: 1, super_admin: 2 };

/** Whether the role opens the admin area. */
export function isStaff(role: Role): boolean {
	return rank[role] >= rank.admin;
}

type Account = Pick<User, "id" | "role">;

function isLocked(viewer: Account, target: Account): boolean {
	return target.role === "super_admin" || target.id === viewer.id;
}

/**
 * Whether the viewer may reset the target's password or delete it. The
 * server enforces the same rule; the admin area only hides what it would
 * refuse.
 */
export function canManage(viewer: Account, target: Account): boolean {
	return !isLocked(viewer, target) && rank[viewer.role] > rank[target.role];
}

export function canChangeRole(viewer: Account, target: Account): boolean {
	return !isLocked(viewer, target) && viewer.role === "super_admin";
}

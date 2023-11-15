
# Policies for User based on their role

allow(actor, _action, _resource) if actor.is_admin();

allow(actor: User, "update", "email") if actor.is_user();

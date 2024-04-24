// TODO:  Taken from oauth2 crate.  Find a common crate to provide this functionality.

#[macro_export]
macro_rules! new_secret_type {
    // Basic struct generation with optional attributes
    (
        $(#[$attr:meta])*
        $name:ident($type:ty)
    ) => {
        new_secret_type![
            $(#[$attr])*
            $name($type)
            impl {}
        ];
    };

    // Struct generation with custom implementations
    (
        $(#[$attr:meta])*
        $name:ident($type:ty)
        impl {
            $($item:tt)*
        }
    ) => {
        new_secret_type![
            $(#[$attr])*,
            $name($type),
            concat!(
                "Create a new `",
                stringify!($name),
                "` to wrap the given `",
                stringify!($type),
                "`."
            ),
            concat!("Get the secret contained within this `", stringify!($name), "`."),
            impl {
                $($item)*
            }
        ];
    };

    // Advanced customization with documentation and custom implementation
    (
        $(#[$attr:meta])*,
        $name:ident($type:ty),
        $new_doc:expr,
        $secret_doc:expr,
        impl {
            $($item:tt)*
        }
    ) => {
        $(#[$attr])*
        pub struct $name($type);

        impl $name {
            // Custom implementations, if any
            $($item)*

            // New method with custom documentation
            #[allow(clippy::missing_const_for_fn)]
            #[doc = $new_doc]
            pub fn new(s: $type) -> Self {
                $name(s)
            }

            // Secret method with custom documentation and security warning
            #[allow(clippy::missing_const_for_fn)]
            #[doc = $secret_doc]
            /// # Security Warning
            /// Leaking this value may compromise security
            pub fn secret(&self) -> &$type { &self.0 }
        }

        // Debug trait implementation to prevent accidental logging of secrets
        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, concat!(stringify!($name), "([redacted])"))
            }
        }
    };
}

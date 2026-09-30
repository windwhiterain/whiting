//! Name types for the entries a [`Registry`](crate::Registry) holds.
//!
//! An empty identifier is reserved and must not be constructed.

use std::fmt;

use facet::Facet;

macro_rules! identifier {
    ($(#[$attribute:meta])* $name:ident, $label:literal) => {
        $(#[$attribute])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Facet)]
        #[facet(transparent)]
        pub struct $name(String);

        impl $name {
            #[doc = concat!("Creates a new ", $label, ".")]
            ///
            /// # Panics
            ///
            /// Panics when `name` is empty, because an empty ", $label, " is reserved.
            pub fn new(name: impl Into<String>) -> Self {
                let name = name.into();
                assert!(!name.is_empty(), concat!(stringify!($name), " must not be empty"));
                Self(name)
            }

            #[doc = concat!("Borrows the ", $label, " as a string slice.")]
            pub fn as_str(&self) -> &str {
                &self.0
            }

            #[doc = "Consumes the identifier and returns the owned name."]
            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(name: &str) -> Self {
                Self::new(name)
            }
        }

        impl From<String> for $name {
            fn from(name: String) -> Self {
                Self::new(name)
            }
        }
    };
}

identifier!(
    /// Unique name of a [`Definition`](crate::Definition) within a registry.
    DefinitionIdentifier,
    "definition identifier"
);

identifier!(
    /// Unique name of a [`Schema`](crate::Schema) within a registry.
    SchemaIdentifier,
    "schema identifier"
);

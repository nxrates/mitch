//! MITCH Order message implementation
//!
//! Order messages (`o`) represent order lifecycle events in financial markets,
//! capturing order placement, modification, and cancellation events.

use crate::body::MitchBody;
use crate::common::{
    message_sizes, combine_type_and_side, extract_order_side, extract_order_type, MitchError,
    OrderSide, OrderType,
};
use crate::timestamp::{encode_u48, decode_u48};

/// Order lifecycle event (32 bytes).
///
/// ## Wire layout (little-endian)
///
/// ```text
/// Offset | Field         | Size | Type    | Description
/// -------|---------------|------|---------|--------------------------------------
/// 0      | ticker        | 8    | u64 LE  | Ticker identifier
/// 8      | order_id      | 4    | u32 LE  | Order identifier
/// 12     | price         | 8    | f64 LE  | Order price
/// 20     | qty           | 4    | u32 LE  | Order quantity
/// 24     | type_and_side | 1    | u8      | [0]=side (1b), [7:1]=order type (7b)
/// 25     | expiry        | 6    | u48 LE  | Expiry ms (Unix epoch milliseconds; 0 = GTC)
/// 31     | _pad          | 1    | [u8; 1] | Alignment to 32B boundary
/// ```
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
pub struct Order {
    pub ticker: u64,
    pub order_id: u32,
    pub price: f64,
    pub qty: u32,
    pub type_and_side: u8,
    pub expiry: [u8; 6],
    pub _pad: [u8; 1],
}

impl Order {
    /// Create a new Order message with validation.
    pub fn new(
        ticker: u64,
        order_id: u32,
        price: f64,
        qty: u32,
        order_type: OrderType,
        side: OrderSide,
        expiry_ms: u64,
    ) -> Result<Self, MitchError> {
        let order = Self {
            ticker,
            order_id,
            price,
            qty,
            type_and_side: combine_type_and_side(order_type, side),
            expiry: encode_u48(expiry_ms),
            _pad: [0; 1],
        };
        order.validate()?;
        Ok(order)
    }

    /// Pack Order into bytes using zero-copy transmutation.
    pub fn pack(&self) -> [u8; message_sizes::ORDER] {
        unsafe { core::mem::transmute(*self) }
    }

    /// Unpack Order from bytes using a zero-copy read.
    pub fn unpack(bytes: &[u8]) -> Result<Self, MitchError> {
        if bytes.len() < message_sizes::ORDER {
            return Err(MitchError::BufferTooSmall {
                expected: message_sizes::ORDER,
                actual: bytes.len(),
            });
        }
        let order = unsafe { (bytes.as_ptr() as *const Self).read_unaligned() };
        order.validate()?;
        Ok(order)
    }

    /// Get the order type from the combined field.
    pub fn get_order_type(&self) -> OrderType {
        extract_order_type(self.type_and_side)
    }

    /// Get the order side from the combined field.
    pub fn get_order_side(&self) -> OrderSide {
        extract_order_side(self.type_and_side)
    }

    /// Get expiry timestamp as u64 from u48 bytes (in milliseconds).
    pub fn get_expiry(&self) -> u64 {
        let bytes = self.expiry;
        decode_u48(&bytes)
    }

    /// Set expiry timestamp from u64 (in milliseconds).
    pub fn set_expiry(&mut self, expiry_ms: u64) {
        self.expiry = encode_u48(expiry_ms);
    }

    /// Check if this is a Good Till Cancel (GTC) order.
    pub fn is_gtc(&self) -> bool {
        self.get_expiry() == 0
    }

    /// Check if this order has expired at a given timestamp.
    pub fn is_expired(&self, current_time_ms: u64) -> bool {
        let expiry = self.get_expiry();
        expiry != 0 && current_time_ms > expiry
    }

    /// Validate the contents of the Order message.
    pub fn validate(&self) -> Result<(), MitchError> {
        if self.ticker == 0 {
            return Err(MitchError::InvalidFieldValue("ticker cannot be zero".into()));
        }
        if self.order_id == 0 {
            return Err(MitchError::InvalidFieldValue("order_id cannot be zero".into()));
        }

        match self.get_order_type() {
            OrderType::Market | OrderType::Limit | OrderType::Stop => {
                if self.price <= 0.0 {
                    return Err(MitchError::InvalidFieldValue("price must be positive for this order type".into()));
                }
                if self.qty == 0 {
                    return Err(MitchError::InvalidFieldValue("qty must be positive for this order type".into()));
                }
            }
            OrderType::Cancel => {} // No price/qty validation for Cancel orders
        }

        Ok(())
    }

    /// Check if this is a buy order.
    pub fn is_buy(&self) -> bool {
        matches!(self.get_order_side(), OrderSide::Buy)
    }

    /// Check if this is a sell order.
    pub fn is_sell(&self) -> bool {
        matches!(self.get_order_side(), OrderSide::Sell)
    }

    /// Calculate the notional value (price * qty).
    pub fn notional_value(&self) -> f64 {
        self.price * self.qty as f64
    }

    /// Get the size of the Order struct in bytes.
    pub const fn size() -> usize {
        message_sizes::ORDER
    }
}

// SAFETY: Order is `#[repr(C, packed)]` with only POD fields; no padding bytes.
unsafe impl MitchBody for Order {
    const SIZE: usize = message_sizes::ORDER;
}

// Compile-time size assertion
const _: () = assert!(core::mem::size_of::<Order>() == message_sizes::ORDER, "Order must be exactly 32 bytes");

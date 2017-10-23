use connect::Connect;
use config::PartialConfig;


/// Start configuring pool by providing a connector
pub fn pool_for<C: Connect>(connector: C)
    -> PartialConfig<C>
{
    PartialConfig {
        connector: connector,
    }
}

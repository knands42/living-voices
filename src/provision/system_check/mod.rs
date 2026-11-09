mod gpu;

use crate::config::SupportedModel;
pub use gpu::{Gpu, GraphicsCard};

const COMFORTABLE_MIB: u64 = 16 * 1024;
const TIGHT_MIB: u64 = 8 * 1024;

/// What the card can carry, in the only terms the first-run gate needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaRecommendation {
    Comfortable,
    Tight,
    Limited,
}

impl Gpu {
    pub fn media_recommendation(&self) -> MediaRecommendation {
        match self.vram_mb.unwrap_or(0) {
            vram_mb if vram_mb >= COMFORTABLE_MIB => MediaRecommendation::Comfortable,
            vram_mb if vram_mb >= TIGHT_MIB => MediaRecommendation::Tight,
            _ => MediaRecommendation::Limited,
        }
    }
}

impl MediaRecommendation {
    pub fn workflow(self) -> SupportedModel {
        match self {
            Self::Comfortable => SupportedModel::FluxQ8,
            Self::Tight | Self::Limited => SupportedModel::FluxQ4,
        }
    }

    pub fn suggests_engine(self) -> bool {
        !matches!(self, Self::Limited)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::SupportedModel;

    fn gpu_with(vram_mb: Option<u64>) -> Gpu {
        Gpu {
            card: GraphicsCard::Nvidia,
            vram_mb,
        }
    }

    #[test]
    fn sixteen_gib_and_up_runs_the_heavier_graph() {
        // given a card that clears the comfortable line
        // when
        let recommendation = gpu_with(Some(16384)).media_recommendation();

        // then
        assert_eq!(recommendation, MediaRecommendation::Comfortable);
        assert_eq!(recommendation.workflow(), SupportedModel::FluxQ8);
        assert!(recommendation.suggests_engine());
    }

    #[test]
    fn eight_to_sixteen_gib_still_offers_the_engine_on_the_lighter_graph() {
        // given the 8 and 12 GiB cards
        // when
        // then
        for vram_mb in [8192, 12288] {
            let recommendation = gpu_with(Some(vram_mb)).media_recommendation();
            assert_eq!(recommendation, MediaRecommendation::Tight);
            assert_eq!(recommendation.workflow(), SupportedModel::FluxQ4);
            assert!(recommendation.suggests_engine());
        }
    }

    #[test]
    fn below_eight_gib_or_no_answer_keeps_the_engine_off_by_default() {
        // given a small card and a machine with no reading at all
        // when
        // then
        for vram_mb in [Some(4096), Some(0), None] {
            let recommendation = gpu_with(vram_mb).media_recommendation();
            assert_eq!(recommendation, MediaRecommendation::Limited);
            assert_eq!(recommendation.workflow(), SupportedModel::FluxQ4);
            assert!(!recommendation.suggests_engine());
        }
    }
}
